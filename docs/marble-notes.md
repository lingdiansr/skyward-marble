# marble 上的排查记录（2026-11-10）

本文件记录在 Redmi Note 12 Turbo（`marble`）上运行 Skyward 时遇到的三个问题、
证据、已做的改动和仍未解决的部分。**未提交**，改动只在本工作区。

## 0. 环境与基线

| 项目 | 值 |
| --- | --- |
| 设备 | Redmi Note 12 Turbo（`marble`），bootloader 已解锁 |
| 安装的系统 | 移植版 HyperOS `OS4.0.6.0.XPCCNXM`（system SDK 37） |
| vendor | SDK 32，`ro.vndk.version=32` |
| 官方固件基准 | `OS3.0.5.0.VMRCNXM`（A15，sdk 35），`skyward-pack stock` 的下载来源 |
| 本仓库 | `lingdiansr/skyward-marble`，分支 `marble` |
| 上游 | `kirklin/skyward` `main` = `ff1668f`（与本仓库 `origin/main` 相同） |

设备分区里**没有** `init_boot_a/_b`（见 §2）。

---

## 1. 启动槽位页“无法读取启动槽位”（已修，未提交）

### 现象

启动槽位页报：

```
android.hardware.boot@1.2::IBootControl is not running;
the phone's boot control has to be started first
```

### 证据

```
# 服务每秒退出一次
skyward: /vendor/bin/hw/android.hardware.boot@1.2-service: pid 2197 ended: exit status: 1

# logcat 里真正的原因
E HidlServiceManagement: Failed to dlopen android.hardware.boot@1.0-impl-1.2-qti.so:
  dlopen failed: cannot locate symbol
  "_ZN7android4base8BasenameERKNSt3__112basic_stringIcNS1_11char_traitsIcEENS1_9allocatorIcEEEE"
  referenced by "/system/lib64/libboot_control_qti.so"
E LegacySupport: Could not get passthrough implementation for android.hardware.boot@1.0::IBootControl/default.

# 20 秒后 ready() 超时
skyward: devices not brought up: /vendor/bin/hw/android.hardware.boot@1.2-service
  registered no android.hardware.boot@1.2::IBootControl/default in 20 seconds
```

### 根因

- 该服务的 `LD_LIBRARY_PATH`（`devices/marble.toml` 原值）把已安装系统的
  `/mnt/system/system/lib64`（A17）排在 recovery 自带 `/system/lib64`（A12/A15）之前。
- `libboot_control_qti.so` 需要 `android::base::Basename(std::string const&)`；
  - `/mnt/system/system/lib64/libbase.so` 只导出 `Basename(std::string_view)`；
  - `/system/lib64/libbase.so` 与 `/apex/com.android.vndk.v32/lib64/libbase.so`
    才导出旧签名。
- 于是 dlopen 失败，服务退出，永不注册；`BootControl::open()` 报 “not running”。

### 改动（`devices/marble.toml`，启动槽位服务一段）

```toml
libraries = ["/vendor/lib64", "/apex/com.android.vndk.v32/lib64", "/system/lib64",
             "/mnt/system/system/lib64/bootstrap", "/mnt/system/system/lib64"]
```

（先 vendor / VNDK / recovery 自带库，已安装系统库只作兜底。）

### 状态

已编入镜像并刷入；**尚未在新镜像里手动复核槽位页 UI**。

---

## 2. 首页没有“双系统”入口（结论：不支持，未实现）

### 依据

`crates/skyward/src/ui.rs:1180`：

```rust
app.state.dual_boot = app.state.recovery_partitions && both("boot")? && both("init_boot")?;
```

`crates/skyward-ui/src/app.rs:1918`：`Screen::Systems => self.dual_boot`，所以入口直接隐藏。

### 3.0.5 官方包的事实

从 `~/Downloads/marble-ota_full-OS3.0.5.0.VMRCNXM-user-15.0-727afd7728.zip`
解出 `payload.bin` 的 manifest（33 个分区）：

```
abl aop aop_config bluetooth boot cpucp devcfg dsp dtbo featenabler hyp imagefv
keymaster mi_ext modem odm product qupfw recovery shrm system system_ext tz uefi
uefisecapp vbmeta vbmeta_system vendor vendor_boot vendor_dlkm xbl xbl_config xbl_ramdump
```

**没有 `init_boot`**。`boot.img` 是 header v4，kernel 46,854,828 B + generic ramdisk
1,380,093 B（`init`、`first_stage_ramdisk/`、`debug_ramdisk`、`system/etc/ramdisk/build.prop`）。
包里的 `recovery.img` sha256 = `707cc936…0303`，与 `devices/marble.toml` 的 `[stock] recovery` 一致。

### 结论

marble 即使在官方 A15 也没有 init_boot，Skyward 的双系统设计（kernel 进 `boot`、
启动程序进 `init_boot`）在这台机器上没有落点。**未实现** boot-only 变体；
若要支持，需要 `skyward-systems` 增加“程序写进 boot 的 ramdisk（保留 kernel）”
一条路径（涉及 `boot.rs` / `store.rs` / `install.rs` / `switch.rs` / `ui.rs` 门控），
并需要第二系统安装包才能真机验证。

---

## 3. WLAN 打不开（进行中，当前卡在 SIGSEGV）

按发现顺序记录，三层已经修好，第四层未解决。

### 3.1 第一次：mount persist EBUSY（已修）

```
无法开启 WLAN: mount /dev/block/by-name/persist on /skyward/wlan/persist as erofs:
  Device or resource busy (os error 16)
```

原因：`[partitions] mounted = ["persist"]` 已把 persist 以 ext4 **rw** 挂在
`/mnt/vendor/persist`；`wlan.rs::place()` 又对它 `mount_ro`。`mount_ro()` 先试
`erofs`，内核在 `get_tree_bdev` 打开已被 ext4 占用的 bdev 时返回 `EBUSY`
（dmesg：`/dev/block/by-name/persist: Can't open blockdev`），而 `EBUSY` 不在
`mount_ro()` 的 `EINVAL|ENODEV` 重试名单里，直接失败；即使先试 ext4 也会 EBUSY。

改动（`crates/skyward/src/wlan.rs::place()`）：先查 `points_of(devnum(device))`，
已有挂载且挂载点里确实存在该文件就直接读，否则才走原来的 mount_ro + umount。

> 同样的修法见 `CLF184/skyward` 的提交 `47c6eb6f fix(piano): reuse mounted persist
> for WLAN firmware`（Xiaomi Pad 8 Pro 的 fork），以及 `mengguizzz/skyward` 的
> `pipa` 分支。上游 `kirklin/skyward` 未修、无相关 issue。

### 3.2 第二次：wpa_supplicant 立即退出（sk_dup，已修）

```
无法开启 WLAN: wpa_supplicant ended as it started: exit status: 1
# 手动跑（带 LD_LIBRARY_PATH）：
CANNOT LINK EXECUTABLE "/vendor/bin/hw/wpa_supplicant":
  cannot locate symbol "sk_dup" referenced by "/vendor/bin/hw/wpa_supplicant"...
```

证据：

| 文件 | 导出 `sk_dup` |
| --- | --- |
| `/mnt/system/system/lib64/libcrypto.so`（A17） | ✘ |
| `/apex/com.android.vndk.v32/lib64/libcrypto.so`（A12） | ✅ |
| `/system/lib64/libcrypto.so`（recovery 自带，A15） | ✅ |

直接调换 `[wlan] libraries` 顺序会把 A17 的 `libbinder_ndk/libbinder/libc++`
与旧库混用，链接失败（`__libcpp_verbose_abort`、`BBinder::setTransactionCodeMap`）。

改动：`wlan.rs` 新增 `crypto_folder()`，在 `/skyward/run/supplicant/libs` 里为
`libcrypto.so`、`libssl.so` 建符号链接（优先 VNDK 目录，其次 `/system/lib64`），
并把该目录放到 `LD_LIBRARY_PATH` 最前；其余库仍用已安装系统的。

### 3.3 第三次：HIDL 服务描述未发布（已修）

```
hwservicemanager: getTransport: Cannot find entry
  android.hardware.wifi.supplicant@1.4::ISupplicant/default
  in either framework or device VINTF manifest.
wpa_supplicant: Failed to initialize wpa_supplicant
```

原因：`secure.rs::describes()` 只认 AIDL 名（`package.Interface/instance`）：
对 HIDL 名 `package@1.4::Interface/instance` 用 `rsplit_once('.')` 会把版本号当成
分隔符，得到错误的 package；且 HIDL 的 manifest XML 用
`<name>ISupplicant</name>` + `<instance>default</instance>` 两个元素，而不是
AIDL 的 `<fqname>IFoo/default</fqname>`。

改动：`describes()` 同时支持两种命名与两种 XML 形态。

修复后 `/system/etc/vintf/manifest/` 里同时出现了
`android.hardware.security.keymint-service-qti-rkp.xml` 和
`android.hardware.wifi.supplicant.xml`。

### 3.4 当前：wpa_supplicant 启动后 SIGSEGV（未解决）

```
I wpa_supplicant: Register Vendor HIDL default service
I wpa_supplicant: Successfully initialized wpa_supplicant
I wpa_supplicant: rfkill: Cannot open RFKILL control device
E wpa_supplicant: eap_proxy: eap_proxy_init
skyward: the WLAN did not come up: wpa_supplicant ended as it started: signal: 11 (SIGSEGV)
```

已做的试验（手动执行，`timeout 3`，rc=139 表示段错误）：

| LD_LIBRARY_PATH 组合 | 结果 |
| --- | --- |
| shim + bootstrap + /mnt/system + /system + vndk + /vendor | SIGSEGV |
| shim + /mnt/system + /system + vndk + /vendor（去掉 bootstrap） | SIGSEGV |
| shim + /mnt/system + vndk + /vendor | SIGSEGV |
| shim + vndk + /vendor + /mnt/system + /system（vndk 优先） | 链接失败：`liblog.so` 需要 `__libcpp_verbose_abort` |

结论：

- 已安装系统（A17）的 `libbinder_ndk.so` 只有它自己提供，且需要 A17 的
  `libbinder` + `libc++`；所以不能把整套 A12/A15 库放到前面。
- A12 vendor 的 `wpa_supplicant` 与 A17 平台库混用时，在 `eap_proxy_init`
  之后崩溃（疑似 HIDL/QMI/libc++ ABI 混用）。
- 恢复环境没有 `debuggerd`，`/data/tombstones` 里只有安装系统时期的旧崩溃，
  本次段错误没有 tombstone。

下一步（未执行）：

1. 用 core dump 拿 backtrace：`echo /sdcard/core.%p > /proc/sys/kernel/core_pattern`
   后复跑，拉回 core，用主机 `gdb`（可用）配合拉回的库看栈。
2. 若崩溃在 `eap_proxy`（QMI），尝试让 `wpa_supplicant` 不初始化 EAP proxy
   （配置文件/编译项，需确认该 vendor 版本是否可关）。
3. 或者反过来：给 A12 vendor 栈补齐缺失的同代库（`libbinder_ndk`、`libbinder`、
   `libc++` 在 recovery 里只有 A17 版），做成一个只含这些的 shim 目录。
4. 最后手段：为 marble 单独准备一套与 vendor 同代（A12）的 supplicant 依赖库
   并随镜像分发。

### 3.5 根因与备选实现（2026-11-11）

段错误与 `eap_proxy`/QMI 无关，是两代 bionic 混用：

- 进程入口是 recovery 自带的 `/system/bin/linker64`（Android 12L，SDK 32）；
- 原 `[wlan] libraries` 把已安装系统（A17）的 `bootstrap/libc.so` 放在最前，
  于是 12L 链接器装入 A17 libc；
- 两代 `TlsModule` 表项不同：12L 为 0x38 字节，`init_ptr/init_size/static_offset`
  在 +0x10/+0x18/+0x20；A17 为 0x40 字节，在 +0x18/+0x20/+0x28（`TlsAlign.skew`）；
- `eapol_sm_init` 里第一次 `pthread_create` 调 `__init_static_tls`，A17 libc
  按自己的偏移读 12L 链接器写的表，得到 `memcpy(dst, NULL, 0x2ff0)`，
  fault address 0x0，即 `signal: 11`。

最小 pthread 程序复现：12L 链接器 + A17 libc 段错误；12L 链接器 + 12L libc
正常；A17 链接器 + A17 libc 正常。

备选按“supplicant 回到 recovery 自己的 12L 链接器/libc”实现。stock recovery
的 12L 用户空间没有 `libbinder_ndk.so`，原厂 12L system 镜像也不在本机；
现有 15.0 包里能取到的最接近的一套平台库放在
`devices/marble-extra/system/lib64/a15/`，只加进 supplicant 的
`LD_LIBRARY_PATH`（`/system/lib64/a15` 在最前），recovery 本身仍用 12L。
库清单与来源见 `devices/marble-extra.README.md`。

配置与代码：

- `[wlan] libraries = ["/system/lib64/a15", "/system/lib64", "/vendor/lib64"]`；
- 删除 `crypto_folder()`：12L 的 `/system/lib64/libcrypto.so` 仍有 `sk_dup`；
- `skyward-pack recovery` 增加 `--extra <dir>`，按目录内路径加入 ramdisk。

验证：用同一套 `LD_LIBRARY_PATH`，`wpa_supplicant -h` 能完整链接；最小
pthread 程序正常；真实 supplicant 启动到 HIDL 初始化后因当前镜像尚未发布
wifi manifest 而退出（退出码 255），**不再出现 SIGSEGV**。端到端 WLAN
需要在含 §3.3 修复的新镜像上刷入后验证。

---

## 4. 当前未提交改动

| 文件 | 内容 |
| --- | --- |
| `devices/marble.toml` | 启动槽位服务的 `libraries` 顺序（§1）；`[wlan] libraries` 改为 `/system/lib64/a15`、`/system/lib64`、`/vendor/lib64`（§3.5） |
| `crates/skyward/src/wlan.rs` | `place()` 复用已挂载的 persist（§3.1）；删除 `crypto_folder()` 与前置 libcrypto/libssl（§3.5） |
| `crates/skyward/src/secure.rs` | `describes()` 支持 HIDL 名与 HIDL manifest XML（§3.3） |
| `crates/skyward-pack/src/main.rs` | `recovery` 新增 `--extra <dir>`（§3.5） |
| `devices/marble-extra/` | 15.0 系统里 supplicant 需要的平台库，来源见 `devices/marble-extra.README.md` |

镜像：`target/skyward-marble-extra.img`（sha256 `701212ff…cc14d`），由 §5
的命令构建。设备当前实际运行的是 `/sdcard/skyward-marble.img`
（sha256 `e5d24a11…`），不含 §3.3/§3.5；刷入前先核对分区哈希。

## 5. 构建与刷入命令

```bash
export ANDROID_NDK_HOME=$HOME/android-sdk/ndk/29.0.14206865
cargo ndk -t arm64-v8a --platform 30 clippy --locked -p skyward -- -D warnings
cargo ndk -t arm64-v8a --platform 30 build --locked --release -p skyward
cargo build --release -p skyward-pack

./target/release/skyward-pack recovery \
  --stock target/stock/marble/recovery.img \
  --binary target/aarch64-linux-android/release/skyward \
  --system-init target/system-init/aarch64-linux-android/release/system-init \
  --config devices/marble.toml \
  --font resources/fonts/MiSansVF.ttf --mono resources/fonts/KirkMono-Regular.ttf \
  --mono-license resources/fonts/KirkMono-OFL.txt \
  --extra devices/marble-extra \
  --out target/skyward-marble.img

adb reboot bootloader
fastboot flash recovery_a target/skyward-marble.img
fastboot reboot recovery
```

## 6. 观察点

```bash
adb shell 'sha256sum /system/bin/recovery'                       # 应为镜像里的二进制
adb shell 'ls /system/etc/vintf/manifest/'                       # 应含 wifi.supplicant.xml
adb shell 'ls -l /system/lib64/a15/'                             # 应含 libbinder_ndk.so 等
adb shell 'dmesg | grep -iE "skyward|wpa_supplicant|qca|cnss" | tail -40'
adb shell 'ps -A | grep -E "wpa_supplicant"'
adb shell 'ls -la /skyward/run/supplicant/sockets/'
```
