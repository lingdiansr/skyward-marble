//! What an update that stopped leaves behind, and taking it away: the
//! copy-on-write images it wrote on the data partition, the state the
//! snapshot machinery keeps, and the logs of it. The folders stay, where
//! the next update looks for them; what is in them goes.

use anyhow::{bail, Context, Result};
use crate::mount;
use std::fs;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::thread::{self, JoinHandle};

/// The folders an update that stopped writes in. Each is kept and what is
/// in it taken away.
const FOLDERS: [&str; 6] = [
    "/data/gsi/ota",
    "/data/ota",
    "/data/ota_package",
    "/data/misc/update_engine/prefs",
    "/data/misc/update_engine_log",
    "/metadata/ota/snapshots",
];

/// The files it leaves outright.
const FILES: [&str; 1] = ["/metadata/ota/state"];

/// Where the data partition is mounted; what an update writes lives there.
const DATA: &str = "/data";

/// Where the snapshot state is kept: the recovery mounts it read-only, so
/// it is opened to writing before anything under it is taken away.
const METADATA: &str = "/metadata";

/// Take away what an update that stopped left, and say how many bytes that
/// was. Internal storage has to be open: what it wrote lives there.
pub fn clear() -> Result<u64> {
    if !mount::is_mount_point(Path::new(DATA))? {
        bail!("internal storage is not open");
    }
    let metadata = Path::new(METADATA);
    if mount::is_mount_point(metadata)? {
        mount::remount_writable(metadata, true)?;
    }
    let mut freed = 0;
    for folder in FOLDERS {
        freed += empty(Path::new(folder))?;
    }
    for file in FILES {
        freed += take(Path::new(file))?;
    }
    Ok(freed)
}

/// Take away what is in `folder`, which stays, and say how many bytes that
/// was.
fn empty(folder: &Path) -> Result<u64> {
    let entries = match fs::read_dir(folder) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(e) => return Err(e).with_context(|| format!("look in {}", folder.display())),
    };
    let mut freed = 0;
    for entry in entries {
        let entry = entry.with_context(|| format!("look in {}", folder.display()))?;
        let path = entry.path();
        freed += size(&path)?;
        let kind = entry.file_type().with_context(|| format!("look at {}", path.display()))?;
        let gone = if kind.is_dir() { fs::remove_dir_all(&path) } else { fs::remove_file(&path) };
        gone.with_context(|| format!("take away {}", path.display()))?;
    }
    Ok(freed)
}

/// Take away `file` and say how many bytes it was; one that is not there
/// is nothing.
fn take(file: &Path) -> Result<u64> {
    let measured = size(file)?;
    match fs::remove_file(file) {
        Ok(()) => Ok(measured),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(0),
        Err(e) => Err(e).with_context(|| format!("take away {}", file.display())),
    }
}

/// How many bytes `path` holds, the folders in it counted as well.
fn size(path: &Path) -> Result<u64> {
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(e) => return Err(e).with_context(|| format!("look at {}", path.display())),
    };
    if !meta.is_dir() {
        return Ok(meta.len());
    }
    let mut total = 0;
    for entry in fs::read_dir(path).with_context(|| format!("look in {}", path.display()))? {
        total += size(&entry.with_context(|| format!("look in {}", path.display()))?.path())?;
    }
    Ok(total)
}

/// The clearing under way on a thread of its own, so the screen keeps up:
/// an update that wrote gigabytes on the data partition takes a while to
/// take away.
pub struct Cleaner {
    ended: Receiver<Result<u64, String>>,
    wake: OwnedFd,
    thread: Option<JoinHandle<()>>,
}

impl Cleaner {
    pub fn start() -> Result<Cleaner> {
        let mut fds = [0; 2];
        if unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC | libc::O_NONBLOCK) } != 0 {
            return Err(std::io::Error::last_os_error()).context("make the clearing thread's wake pipe");
        }
        let (wake, signal) = unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) };
        let (tx, ended) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("cache".into())
            .spawn(move || {
                let _ = tx.send(clear().map_err(|e| format!("{e:#}")));
                unsafe { libc::write(signal.as_raw_fd(), [1u8].as_ptr().cast(), 1) };
            })
            .context("start the clearing thread")?;
        Ok(Cleaner { ended, wake, thread: Some(thread) })
    }

    pub fn fd(&self) -> RawFd {
        self.wake.as_raw_fd()
    }

    /// How the clearing ended, once it has.
    pub fn ended(&self) -> Option<Result<u64, String>> {
        let mut drain = [0u8; 8];
        while unsafe { libc::read(self.wake.as_raw_fd(), drain.as_mut_ptr().cast(), drain.len()) } > 0 {}
        self.ended.try_recv().ok()
    }
}

impl Drop for Cleaner {
    /// The screen lets the thread go once it has ended.
    fn drop(&mut self) {
        if let Some(thread) = self.thread.take() {
            thread.join().expect("the clearing thread does not panic");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder of its own under the temporary folder, empty.
    fn workspace(name: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("skyward-cache-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn emptying_a_folder_takes_everything_in_it_and_says_how_much() {
        let root = workspace("empty");
        let folder = root.join("ota");
        fs::create_dir_all(folder.join("nested")).unwrap();
        fs::write(folder.join("a"), vec![0u8; 1000]).unwrap();
        fs::write(folder.join("nested/b"), vec![0u8; 2000]).unwrap();
        assert_eq!(empty(&folder).unwrap(), 3000, "every byte under it, the folders too");
        assert_eq!(fs::read_dir(&folder).unwrap().count(), 0, "the folder stays, with nothing in it");
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_file_that_is_not_there_is_nothing_to_take_away() {
        let root = workspace("missing");
        assert_eq!(take(&root.join("state")).unwrap(), 0);
        fs::write(root.join("state"), vec![0u8; 40]).unwrap();
        assert_eq!(take(&root.join("state")).unwrap(), 40);
        assert!(!root.join("state").exists());
        fs::remove_dir_all(&root).unwrap();
    }
}
