# marble: platform libraries the stock recovery does not carry

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
