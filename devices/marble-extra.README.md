# marble: what the stock recovery does not carry

`system/lib64/a15/` holds the AOSP platform libraries the vendor
`wpa_supplicant` needs but the stock OS3.0.5.0 recovery does not carry.

The recovery's own user space is an Android 12L build (SDK 32), the same
generation as the vendor partition. It has no `libbinder_ndk.so`, and the
12L system image that would match it is not in the stock package or on the
phone. These files are the closest shipped set: the `system` partition of
the 15.0 package.

They are copied from

    marble-ota_full-OS3.0.5.0.VMRCNXM-user-15.0-727afd7728.zip
      -> payload.bin -> system.img (erofs)

at `system/lib64/`, and are only on the supplicant's `LD_LIBRARY_PATH`
(`/system/lib64/a15`) ahead of the recovery's own 12L libraries; the
recovery itself keeps its 12L linker and libc.

`skyward-pack recovery --extra devices/marble-extra` adds this tree to the
ramdisk at the paths it lies at. Modes are 0644, owned by root.

Files (all from `system/lib64` of the 15.0 system):

    libapexsupport.so libbase.so libbinder.so libbinder_ndk.so
    libc++.so libcutils.so libdl_android.so liblog.so
    libunwindstack.so libutils.so libutilscallstack.so libvndksupport.so

Licence: Apache-2.0 (AOSP). Keep this notice with the files.

Regenerate:

    go build -o /tmp/payload-dumper-go github.com/ssut/payload-dumper-go
    unzip -p marble-ota_full-OS3.0.5.0.VMRCNXM-*.zip payload.bin > payload.bin
    /tmp/payload-dumper-go -p system -o out payload.bin
    erofsfuse out/system.img /tmp/system
    cp /tmp/system/system/lib64/<name> devices/marble-extra/system/lib64/a15/

`erofsfuse` comes from erofs-utils (`git.kernel.org/pub/scm/linux/kernel/git/xiang/erofs-utils.git`).

# ntfs-3g: NTFS read in the phone's own user space

`system/bin/ntfs-3g` is ntfs-3g 2026.9.28, built statically against the
recovery's own user space so nothing else has to be carried with it.

The kernel these phones run (5.10 on marble) has no NTFS driver at all:
`# CONFIG_NTFS_FS is not set`, and ntfs3 only exists from 5.15 on. Android's
system does not use a kernel driver for NTFS either — `vold` runs
`/system/bin/mount.ntfs` (MIUI 13) or `/system_ext/bin/mount.ntfs` (the 15.0
build), which is ntfs-3g over FUSE. The recovery does the same, through
`skyward_block::usb::mount`, after the kernel has said no to every
filesystem it knows.

Built with the Android NDK's API 32 toolchain, the generation of the
recovery's own user space:

    curl -LO https://codeload.github.com/tuxera/ntfs-3g/tar.gz/refs/tags/2026.9.28
    tar xzf 2026.9.28.tar.gz && cd ntfs-3g-2026.9.28
    ./autogen.sh

    NDK=~/Android/Sdk/ndk/29.0.14206865
    PATH=$NDK/toolchains/llvm/prebuilt/linux-x86_64/bin:$PATH \
    CC=aarch64-linux-android32-clang AR=llvm-ar RANLIB=llvm-ranlib \
    ./configure --host=aarch64-linux-android --build=x86_64-pc-linux-gnu \
      ac_cv_lib_pthread_pthread_create=yes \
      --prefix=/usr --exec-prefix=/usr --sbindir=/usr/bin --bindir=/usr/bin \
      --disable-ldconfig --disable-shared --enable-static \
      --with-fuse=internal --enable-mount-helper \
      --disable-ntfsprogs --disable-crypto
    sed -i 's/ -lpthread//g' $(find . -name Makefile -o -name '*.la')
    make -j$(nproc)
    llvm-strip src/ntfs-3g
    cp src/ntfs-3g devices/marble-extra/system/bin/ntfs-3g

Two things are needed beyond upstream: bionic keeps its thread functions in
libc and ships no libpthread, so the pthread check is answered by
`ac_cv_lib_pthread_pthread_create=yes`; and the internal FUSE library names
`-lpthread` in the dependency list libtool writes, which is taken out before
linking.

Licence: GPL-2.0-or-later. The source of exactly this build is the tarball
above. Keep this notice with the file.
