![Skyward Recovery](assets/hero.jpg)

# Skyward

English | [简体中文](README.zh-CN.md)

Skyward is a recovery written in Rust, from scratch.

<p align="center">
  <img src="assets/home.png" width="30%" alt="Home">
  <img src="assets/settings-dark.png" width="30%" alt="Settings in dark mode">
  <img src="assets/time-zone.png" width="30%" alt="Time zone">
</p>

## This branch: Redmi Note 12 Turbo (marble)

This repository is a fork of [kirklin/skyward](https://github.com/kirklin/skyward)
whose default branch, `marble`, adds the Redmi Note 12 Turbo (Snapdragon 7+ Gen 2
Mobile Platform), built on the Chinese OS3.0.5.0.VMRCNXM package (Android 15).
Upstream's `main` is kept as `main` here. The fork's releases are on
[Releases](https://github.com/lingdiansr/skyward-marble/releases), tagged
after upstream's version with a pre-release suffix such as `v0.2.0-rc.3`; the
image's own update check asks this fork's releases. The image is also built as
[Building](#building) shows and flashed to the recovery partition.

The phone's stock recovery is barer than the ones upstream builds on: no
servicemanager and no binder filesystem, a 12L user space beside a newer
installed system, and none of the platform libraries the vendor's WLAN
supplicant needs. The branch therefore changes both the device configuration
and, where the stock recovery leaves nothing to configure, the recovery itself.

**Touch.** The panel is Goodix's 9916r on SPI, driven by `gtx8_spi` in
`goodix_core.ko`; the other panel drivers the phone's own `modules.load` names
belong to the suppliers another unit of this model may carry and bring up nothing
here. The recovery's init loads `goodix_core` before `xiaomi_touch`, the core it
registers with, so Skyward unloads and reloads it after that. The owner list
holds only the nodes this panel makes: `hand_over` used to stop on one the panel
never creates, which ended `bring_up` before the services and the WLAN started.

**Data decryption.** The stock recovery carries no servicemanager and mounts no
binder filesystem, so nothing answers a key request and data stays locked. Its
ramdisk now mounts binderfs at early-init, and when the recovery has no service
directory of its own Skyward starts the installed system's, speaks its AIDL wire
format (`SYST` in place of the recovery's `RECO`) and its method numbers, and
lets those services link against the installed system's libraries. The credential
service is HIDL on this phone and runs through the installed system's
`hwservicemanager`, which Skyward exposes at its standard path
`/system/bin/hwservicemanager`; `qseecomd`, the daemon the secure world calls
back through, is kept off the installed system's libraries, whose `libxml2` needs
the i18n apex's `libandroidicu.so`.

**Boot control.** The service calls
`android::base::Basename(std::string const&)`, which the installed system's
`libbase` no longer exports, so it exits as it starts and the boot slot page
reports it missing. Its libraries come from the vendor build first and the
installed system's last, and it registers.

**WLAN.** The chip is `qca6490` over PCIe, driven by `qca_cld3_qca6490` over
`cnss2`; its firmware and the MAC address come from the modem and persist
partitions. The device configuration mounts persist read-write already, so
Skyward reuses that mount for the firmware instead of mounting the partition
read-only again, which the kernel refuses on a busy device. The vendor supplicant
is an Android 12L build and runs on the recovery's own 12L linker and libc; the
15.0 platform libraries the stock recovery does not carry are kept under
`/system/lib64/a15` and added to the ramdisk with
`--extra devices/marble-extra`, whose README records where each comes from.
Skyward describes the supplicant's HIDL manifest by its name and instance
elements, so the hardware service directory finds it.

**MTP.** Events answer no command; they are sent with transaction 0, so a host
no longer ignores one that names the last command's transaction, and storage
added while a session is open appears without re-binding.

**Command line.** init starts the recovery service by path with no arguments,
but the same binary carries the command line. Installed as recovery, it runs the
service only when it has no arguments, so `skyward recovery slot`, `bcb` and
`info` still parse.

### Provenance

The port began from upstream's `devices/nuwa.toml`, with
[pubglite55/skyward-for-nuwa](https://github.com/pubglite55/skyward-for-nuwa)
as the reference for a one-device fork. These parts come from other work:

| Measure | Where it came from |
| --- | --- |
| The key service chain — `qseecomd`, KeyMint, the HIDL credential service and `hwservicemanager` — with its users and groups | Upstream [`devices/ishtar.toml`](https://github.com/kirklin/skyward/blob/main/devices/ishtar.toml) (Xiaomi 13 Ultra), adapted here: the library paths, and no wait on `vendor.sys.listeners.registered` |
| The backup's `efs` and skipped data, the interfaces KeyMint registers, and the log folders a wipe clears | Upstream [`devices/nezha.toml`](https://github.com/kirklin/skyward/blob/main/devices/nezha.toml) (Xiaomi 17 Ultra); these lists are the same |
| The binderfs mount at early-init, the installed system's AIDL servicemanager, the `SYST` parcel header and method numbers, and `modules.reload` putting `goodix_core` back after `xiaomi_touch` | [Upstream PR #26](https://github.com/kirklin/skyward/pull/26), Memories-white's Redmi K60 (`mondrian`) recovery, cherry-picked from `feat/mondrian-recovery` at `a84f5f67`; the PR is still open |
| The boot control's library order | Reached here from the missing `Basename` symbol, then checked against PR #26's phone — the same ported system and stock Android 15 recovery, where `IBootControl` registers |
| The WLAN MAC address read from the persist mount that is already there | [CLF184/skyward `47c6eb6f`](https://github.com/CLF184/skyward/commit/47c6eb6ff6ff9c019293b6a8af898b64e8d1faec) (`piano`, Xiaomi Pad 8 Pro) and [mengguizzz/skyward](https://github.com/mengguizzz/skyward)'s `pipa` branch, which fixed it the same way; both are fork-only |

The rest — the Goodix panel and its owner list, the WLAN runtime, the MTP event
and the command line — was worked out on this branch and phone. The VINTF
manifest matching was written here as well, and also covers what open upstream
PR #27 proposes, HIDL names included.

### Releases

A release is tagged `v<upstream version>-rc.<n>`, and the workspace `Cargo.toml`
version names the same string, so the binary and its update check agree. The
workflow builds only this phone for a tag and marks the release a pre-release.

### Limits

- Dual boot: the phone has no `init_boot` partition, the official Android 15
  package either, so upstream's design — the kernel in `boot`, the second
  system's first program in `init_boot` — has no place here. The entry stays
  hidden, and no variant writes the program into `boot`'s ramdisk.
- The image is built on the stock Android 15 package; the installed system the
  phone runs is a ported HyperOS build.

## Features

### Dual boot

![One phone holds two systems](slides/en/out/01-dual-boot.png)

![A switch rewrites only two partitions](slides/en/out/02-dual-boot-partitions.png)

How a second system is packaged and started is in
[How dual boot works](#how-dual-boot-works).

### Decryption

![Decrypt data with the screen lock](slides/en/out/03-decrypt.png)

### Installing

![Four kinds of packages from three places](slides/en/out/04-install.png)

Install scripts get `7za` and `zstd`, which ROMs with a super image compressed by
zstd use to flash it, and an `unzip` whose `-p` streams the image as it reads it.
They also get `avbctl` and `resetprop`, to turn off verified boot's checks and to
read and set properties.

An unlocked bootloader puts one more choice on the Settings page: **Allow
installing an older system**. With it on, a full OTA older than the installed
system installs too, and the data partition is erased once the update is applied;
Skyward refuses an older package while the bootloader is locked. It is off until
it is turned on.

### Online updates

![Update the system and Skyward online](slides/en/out/05-updates.png)

### Backup and restore

![Back up the system, apps and IMEI](slides/en/out/06-backup.png)

### File manager

![Manage the phone's files in Skyward](slides/en/out/07-files.png)

### Terminal

![Run a root shell right on the phone](slides/en/out/08-terminal.png)

### Connecting a computer

![Move files and flash partitions over USB](slides/en/out/09-computer.png)

### Wiping data

![Every choice says what it wipes](slides/en/out/10-wipe.png)

### Replacing the stock recovery

![Skyward does what the system asks](slides/en/out/11-requests.png)

### Search

![Type a few letters to find a feature](slides/en/out/12-search.png)

### Advanced

![When something breaks, fix it here first](slides/en/out/13-tools.png)

### About phone

![Check battery health and charge cycles](slides/en/out/14-specs.png)

### Other features

- Reboot to system, recovery, fastboot, bootloader or EDL, and switch slots
- Light and dark themes; Simplified Chinese, Traditional Chinese and English

### Without a system

![Skyward starts even without a system](slides/en/out/15-no-system.png)

## Supported devices

| Device | Codename | Tested system version |
| --- | --- | --- |
| Xiaomi 17 Ultra | `nezha` | OS4.0.4.0.XPACNXM (Android 17) |
| Xiaomi 13 Ultra | `ishtar` | OS3.0.308.0.WMACNXM (Android 16) |

### 🚧 Under construction

These images are built from each phone's stock firmware and have not been tried
on the phone. They set up the screen, touch, Wi-Fi and vibration, and do not decrypt
the data partition.

| Device | Codename | Built on |
| --- | --- | --- |
| 🚧 Xiaomi 13 | `fuxi` | OS3.0.309.0.WMCCNXM (Android 16) |
| 🚧 Xiaomi 13 Pro | `nuwa` | OS3.0.312.0.WMBCNXM (Android 16) |
| 🚧 Xiaomi 14 | `houji` | OS3.0.307.0.WNCCNXM (Android 16) |
| 🚧 Xiaomi 14 Pro | `shennong` | OS3.0.309.0.WNBCNXM (Android 16) |
| 🚧 Xiaomi 14 Ultra | `aurora` | OS3.0.308.0.WNACNXM (Android 16) |
| 🚧 Xiaomi 15 | `dada` | OS3.0.307.0.WOCCNXM (Android 16) |
| 🚧 Xiaomi 15 Pro | `haotian` | OS3.0.309.0.WOBCNXM (Android 16) |
| 🚧 Xiaomi 15 Ultra | `xuanyuan` | OS3.0.314.0.WOACNXM (Android 16) |
| 🚧 Xiaomi 17 | `pudding` | OS4.0.4.0.XPCCNXM (Android 17) |
| 🚧 Xiaomi 17 Pro | `pandora` | OS4.0.4.0.XBLCNXM (Android 17) |
| 🚧 Xiaomi 17 Pro Max | `popsicle` | OS4.0.4.0.XPBCNXM (Android 17) |
| 🚧 Xiaomi 17 Max | `byron` | OS3.0.310.0.WAFCNXM (Android 16) |
| 🚧 REDMI K80 | `zorn` | OS3.0.309.0.WOKCNXM (Android 16) |
| 🚧 REDMI K80 Pro | `miro` | OS3.0.305.0.WOMCNXM (Android 16) |
| 🚧 REDMI K90 | `annibale` | OS3.0.307.0.WPKCNXM (Android 16) |
| 🚧 REDMI K90 Pro Max | `myron` | OS4.0.4.0.XPMCNXM (Android 17) |
| 🚧 REDMI K90 Ultra | `warsaw` | OS3.0.309.0.WHPCNXM (Android 16) |
| 🚧 REDMI K100 Pro | `athens` | OS4.0.4.0.XPICNXM (Android 17) |
| 🚧 REDMI K100 Pro Max | `songyuan` | OS4.0.4.0.XGNCNXM (Android 17) |

## Installing Skyward

Skyward needs an unlocked bootloader. Download the zip for this phone from the
fork's [Releases](https://github.com/lingdiansr/skyward-marble/releases), or
build the image as [Building](#building) shows. Each zip is named for the phone's
model and codename, such as Redmi Note 12 Turbo (marble), and its version follows
upstream's with a pre-release suffix, such as `v0.2.0-rc.3`. Check it against
`SHA256SUMS`, unzip it, and flash the image inside to the recovery partition of
the slot the phone runs from, with the phone in fastboot mode:

```sh
unzip skyward-v0.2.0-rc.3-marble-OS3.0.5.0.VMRCNXM.zip
fastboot flash recovery skyward-v0.2.0-rc.3-marble-OS3.0.5.0.VMRCNXM.img
fastboot reboot recovery
```

Afterwards Skyward updates itself over Wi-Fi from this fork's newest release, on
the Skyward update page in Settings; a newer image, unzipped, also installs from
its own Install page. With **Flash Skyward again** on, a system update or zip that writes
over the recovery partitions gets Skyward back in them.

adb in Skyward is a root shell that needs no authorization on the phone: anyone
with the phone and a cable can reach its storage while it is in recovery.

## How dual boot works

Skyward installs a second system beside the one in the phone's own partitions,
whether that is HyperOS or a custom ROM, and switches between them from **Dual boot**
on its home page. The second system's root is a file in the data partition that the
filesystem may neither move nor delete, read and written through the partition
itself; the boot images of both systems are kept in another such file. A switch
writes only the boot and init_boot partitions of the slot that starts next, and
keeps each step on /metadata first, so a switch cut off by a loss of power is
finished when Skyward next starts. vendor_boot, dtbo, recovery, vbmeta, the firmware
and super stay as they are, and the second system starts with the installed
system's vendor_boot and dtbo.

Before anything else writes the boot partitions, Skyward puts the installed system's
boot images back: an install, a slot switch, fastboot, a factory reset, and a backup
or restore of the boot partitions. The second system waits while a system update is
still to be merged, and backups of the data partition leave it out.

A second system installs from a zip on the Install page, as any other package does:

| File | What it holds |
| --- | --- |
| `system.toml` | `id`, `name`, `version` and `filesystem`; `init`, the program the root starts with, `/sbin/init` unless given; `minimum`, the least room the root takes, in bytes |
| `boot.img` | The kernel the bootloader starts |
| `rootfs.img` | The root filesystem's image, of the `filesystem` named |
| `init_boot.img` | Only for a system with a first ramdisk of its own |

A system without an init_boot image of its own starts from one Skyward makes. Its
program loads the installed system's first-stage kernel modules the way that
system's first stage loads them, with their soft dependencies and the settings the
kernel command line gives them, finds the root in the data partition, mounts it and
starts `init` there. If anything stops it, it brings Skyward up, as a failed start
of the installed system ends in the recovery: it leaves the reason in misc, unpacks
the ramdisk of the slot's recovery partition over its own, as the kernel unpacks it
when the bootloader starts the recovery, and hands over to Skyward's first program,
so Skyward comes up however the bootloader treats a restart. Skyward shows why, and
the second system still starts next until the user switches on Dual boot. A start
that stops moving restarts into the recovery instead, and a phone restarted before
the hand-over finds the same request in misc at its next start, which brings
Skyward up before anything else. Where Skyward comes up neither way, the phone stays
where it stopped, and never starts the same again and again. A system with a first
ramdisk of its own finds Skyward's program at `/skyward/map`, which lays the root
out as a block device and prints its node. Each switch to the second system writes
its init_boot image with the program of the Skyward that switches, so an updated
Skyward starts it with the new program.

A running system switches by writing `boot-recovery` with
`--switch_system=installed`, or `--switch_system=` and the second system's `id`, to
misc, and restarting into recovery. Where the bootloader starts the slot again, as
the 13 Ultra's can with a USB cable in, Skyward's program finds the request in misc
at the next start and brings Skyward up before anything else.

## Building

Skyward builds with the stable Rust that `rust-toolchain.toml` names, with its
`aarch64-linux-android` target, the Android NDK r29 and
[cargo-ndk](https://github.com/bbqsrc/cargo-ndk). The recovery service is built for
the phone, and the packer for the computer:

```sh
cargo ndk -t arm64-v8a --platform 30 build --release -p skyward
cargo build --release -p skyward-pack
```

The second system's first program runs before any library is there to load, so it
is built as a static program:

```sh
RUSTFLAGS="-C target-feature=+crt-static" cargo ndk -t arm64-v8a --platform 30 build --release -p skyward-system-init --target-dir target/system-init
```

The packer takes the stock recovery partition out of the build's full update
package on Xiaomi's server, reading only the part of the package the partition is
in, and checks its SHA-256 against the device file:

```sh
target/release/skyward-pack stock --config devices/nezha.toml --out target/stock/nezha/recovery.img
```

Then it builds the image on top of it:

```sh
target/release/skyward-pack recovery \
  --stock target/stock/nezha/recovery.img \
  --binary target/aarch64-linux-android/release/skyward \
  --system-init target/system-init/aarch64-linux-android/release/system-init \
  --config devices/nezha.toml \
  --font resources/fonts/MiSansVF.ttf \
  --mono resources/fonts/KirkMono-Regular.ttf \
  --mono-license resources/fonts/KirkMono-OFL.txt \
  --out target/skyward-nezha.img
```

### This branch's image (marble)

The WLAN's platform libraries are not in the stock recovery, so packing the
marble image adds `devices/marble-extra` to the ramdisk; the rest is the steps
above.

```sh
target/release/skyward-pack stock --config devices/marble.toml --out target/stock/marble/recovery.img
target/release/skyward-pack recovery \
  --stock target/stock/marble/recovery.img \
  --binary target/aarch64-linux-android/release/skyward \
  --system-init target/system-init/aarch64-linux-android/release/system-init \
  --config devices/marble.toml \
  --font resources/fonts/MiSansVF.ttf \
  --mono resources/fonts/KirkMono-Regular.ttf \
  --mono-license resources/fonts/KirkMono-OFL.txt \
  --extra devices/marble-extra \
  --out target/skyward-marble.img
```

Flash it to the recovery partition of the slot the phone starts from:

```sh
fastboot flash recovery_a target/skyward-marble.img
fastboot reboot recovery
```

Tests and lints run on the computer; what runs only on the phone is linted for it:

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo ndk -t arm64-v8a --platform 30 clippy -p skyward -p skyward-install -p skyward-systems -p skyward-system-init -- -D warnings
```

`skyward-preview` draws the pages on the computer, as PNG files, without a phone:
each page whole, as a long screenshot shows it, in the states it can be in, and the
screen at moments of touches and motion. `--lang` picks the language by a locale
such as `en` or `zh-TW`:

```sh
cargo run --release -p skyward-preview -- target/preview --lang en
```

## License

[GPL-3.0-or-later](LICENSE)
