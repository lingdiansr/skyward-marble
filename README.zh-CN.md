![Skyward Recovery](assets/hero.jpg)

# Skyward

[English](README.md) | 简体中文

Skyward 是使用 Rust 编写的 recovery，从零开始编写。

<p align="center">
  <img src="assets/home.png" width="30%" alt="首页">
  <img src="assets/settings-dark.png" width="30%" alt="深色模式下的设置">
  <img src="assets/time-zone.png" width="30%" alt="时区">
</p>

## 本分支：Redmi Note 12 Turbo（`marble`）

本仓库是 [kirklin/skyward](https://github.com/kirklin/skyward) 的分支，默认分支
`marble` 加入 Redmi Note 12 Turbo（第二代骁龙 7+ 移动平台）的支持，按照国行
OS3.0.5.0.VMRCNXM 完整包（Android 15）构建。上游的 `main` 在这里仍为 `main`。
本分支不发布 Releases：镜像按[构建](#构建)中的步骤构建，再刷入 recovery 分区。

这台手机的原厂 recovery 比上游构建所基于的 recovery 少了很多东西：没有
servicemanager，也不挂载 binder 文件系统；自身用户空间是 Android 12L，已安装的
系统更新；原厂 supplicant 需要的平台库也一个都没有。本分支因此既改动了设备配置，
也在原厂 recovery 没有可配置之处时改动了 recovery 本身。

**触摸。** 面板是 Goodix 9916r，走 SPI，由 `goodix_core.ko` 里的 `gtx8_spi`
驱动；手机自己的 `modules.load` 里列的其余面板驱动属于同型号其他供应商的机器，
在这里不能启动任何设备。recovery 的 init 在 `goodix_core` 注册所依赖的
`xiaomi_touch` 之前就加载了它，所以 Skyward 先把它卸载，再在 `xiaomi_touch`
之后重新加载。owner 列表只保留这块面板真正会创建的节点：`hand_over` 以前会在
面板不会创建的节点上失败，导致 `bring_up` 在服务和 WLAN 启动之前就结束。

**解密 data。** 原厂 recovery 没有 servicemanager，也不挂载 binder 文件系统，
因此没有程序回应密钥请求，data 一直处于锁定状态。现在 ramdisk 在 early-init
挂载 binderfs；recovery 没有自己的服务目录时，Skyward 启动已安装系统的目录，
使用它的 AIDL 线格式（`SYST`，而不是 recovery 的 `RECO`）和方法号，这些服务
也链接已安装系统的库。这台手机的凭据服务是 HIDL 的，通过已安装系统的
`hwservicemanager` 运行；Skyward 把它放到标准路径
`/system/bin/hwservicemanager`。安全世界回调用的 `qseecomd` 不使用已安装系统的
库，因为它的 `libxml2` 需要 i18n apex 里的 `libandroidicu.so`。

**启动槽位。** 启动槽位服务调用
`android::base::Basename(std::string const&)`，已安装系统的 `libbase` 不再导出
这个签名，服务一启动就退出，槽位页因此报它没有运行。它的库改为先从 vendor 的
构建取、已安装系统的库放在最后，服务就能注册。

**WLAN。** 芯片是 PCIe 上的 `qca6490`，由 `cnss2` 上的 `qca_cld3_qca6490`
驱动；固件和 MAC 地址来自 modem 和 persist 分区。设备配置已经把 persist 以可写
方式挂载，Skyward 直接复用这个挂载点取固件，不再对忙着的设备做只读挂载。原厂
supplicant 是 Android 12L 构建，运行在 recovery 自己的 12L 链接器和 libc 上；
原厂 recovery 没有的 15.0 平台库放在 `/system/lib64/a15`，由
`--extra devices/marble-extra` 加进 ramdisk，来源记录在
`devices/marble-extra.README.md`。supplicant 的 HIDL manifest 按名称和实例两个
元素描述，硬件服务目录因此能找到它。

**MTP。** MTP 事件不需要回应命令；现在都以事务号 0 发送，主机不会再忽略指向
上一个命令事务的事件，会话打开期间新增的存储也会直接出现，不用重新绑定。

**命令行。** init 按路径启动 recovery 服务时不带参数，但同一个程序也带着命令行。
安装为 recovery 之后，只有不带参数时才运行服务，所以 `skyward recovery slot`、
`bcb`、`info` 等命令仍然可以解析。

### 来源

移植以原厂上游的 `devices/nuwa.toml` 为起点，并参考了
[pubglite55/skyward-for-nuwa](https://github.com/pubglite55/skyward-for-nuwa)
这个单机型 fork 的做法。以下部分来自上游或其他工作：

| 措施 | 来源 |
| --- | --- |
| 密钥服务链：`qseecomd`、KeyMint、HIDL 凭据服务和 `hwservicemanager`，连同用户与组 | 上游 [`devices/ishtar.toml`](https://github.com/kirklin/skyward/blob/main/devices/ishtar.toml)（小米 13 Ultra），本分支做了调整：库路径，以及不再等待 `vendor.sys.listeners.registered` |
| 备份的 `efs` 和要跳过的数据、KeyMint 注册的接口，以及清除数据时清掉的日志目录 | 上游 [`devices/nezha.toml`](https://github.com/kirklin/skyward/blob/main/devices/nezha.toml)（小米 17 Ultra）；这些列表与本机一致 |
| early-init 挂载 binderfs、启动已安装系统的 AIDL servicemanager、`SYST` parcel 头与方法号、把 `goodix_core` 放到 `xiaomi_touch` 之后重载的 `modules.reload` | [上游 PR #26](https://github.com/kirklin/skyward/pull/26)，Memories-white 的 Redmi K60（`mondrian`）适配，cherry-pick 自 `feat/mondrian-recovery` 的 `a84f5f67`；该 PR 仍未合并 |
| 启动槽位服务的库顺序 | 由本机缺失的 `Basename` 符号定位，再按 PR #26 的同环境配置核对（同一移植系统 + 原厂 A15 recovery，`IBootControl` 能注册） |
| 从已挂载的 persist 读 WLAN MAC 地址 | [CLF184/skyward `47c6eb6f`](https://github.com/CLF184/skyward/commit/47c6eb6ff6ff9c019293b6a8af898b64e8d1faec)（`piano`，小米平板 8 Pro）与 [mengguizzz/skyward](https://github.com/mengguizzz/skyward) 的 `pipa` 分支，做法相同；上游 PR [#1](https://github.com/kirklin/skyward/pull/1)、[#15](https://github.com/kirklin/skyward/pull/15) 提过同样的改动，均已关闭未合并 |

其余部分——Goodix 面板及其 owner 列表、WLAN 的运行时、MTP 事件和 recovery
命令行——是本分支在真机和代码上做的。VINTF manifest 匹配同样是本分支写的，
覆盖了仍未合并的上游 PR #27 提出的内容，并支持 HIDL 名。

### 限制

- 双系统：这台手机没有 `init_boot` 分区，官方 Android 15 包里也没有，上游的设计
  ——内核写在 `boot`、第二系统的第一个程序写在 `init_boot`——在这里没有落点。
  入口保持隐藏，也没有把程序写进 `boot` 的 ramdisk 的变体。
- 镜像按照原厂 Android 15 完整包构建；真机上的已安装系统是移植的 HyperOS。

## 功能

### 双系统

![一台手机可以安装两个系统](slides/zh-CN/out/01-dual-boot.png)

![切换系统只改写两个分区](slides/zh-CN/out/02-dual-boot-partitions.png)

第二系统的安装包格式和启动过程见下文的[双系统的工作方式](#双系统的工作方式)。

### 解密

![用锁屏图案或密码解密 data 分区](slides/zh-CN/out/03-decrypt.png)

### 安装

![从三个来源安装四种安装包](slides/zh-CN/out/04-install.png)

刷机包的安装脚本可以使用 `7za` 和 `zstd`，支持刷入 super 镜像使用 zstd 压缩的第三方 ROM；`unzip -p` 边解压边输出镜像。安装脚本还可以使用 `avbctl` 关闭验证启动的校验，使用 `resetprop` 读取和设置属性。

### 在线更新

![系统和 Skyward 都能在线更新](slides/zh-CN/out/05-updates.png)

### 备份和恢复

![系统、应用和 IMEI 都可以备份](slides/zh-CN/out/06-backup.png)

### 文件管理

![在 Skyward 中管理手机里的文件](slides/zh-CN/out/07-files.png)

### 终端

![手机上就能运行 root shell](slides/zh-CN/out/08-terminal.png)

### 连接电脑

![用数据线连接电脑，传文件和刷写分区](slides/zh-CN/out/09-computer.png)

### 清除数据

![每一项都写明会清除哪些内容](slides/zh-CN/out/10-wipe.png)

### 替换原厂 recovery

![系统发来的请求，Skyward 照常处理](slides/zh-CN/out/11-requests.png)

### 搜索

![输入拼音首字母，就能找到功能](slides/zh-CN/out/12-search.png)

### 高级

![系统出了问题，可以先在这里修复](slides/zh-CN/out/13-tools.png)

### 关于本机

![电池剩余容量和充电循环都能查看](slides/zh-CN/out/14-specs.png)

### 其他功能

- 重启到系统、recovery、fastboot、bootloader 和 EDL，切换启动槽位
- 浅色和深色模式，简体中文、繁体中文和英文

### 没有系统时

![super 中没有系统，也能装回一个系统](slides/zh-CN/out/15-no-system.png)

## 支持的机型

| 机型 | 代号 | 测试的系统版本 |
| --- | --- | --- |
| 小米 17 Ultra | `nezha` | OS4.0.4.0.XPACNXM（Android 17） |
| 小米 13 Ultra | `ishtar` | OS3.0.308.0.WMACNXM（Android 16） |

### 🚧 施工中

这些机型的镜像按照各自的原厂固件构建，还没有在真机上试过。屏幕、触摸、WLAN 和振动已经配置好，不会解密 data 分区。

| 机型 | 代号 | 构建所用的系统版本 |
| --- | --- | --- |
| 🚧 小米 13 | `fuxi` | OS3.0.309.0.WMCCNXM（Android 16） |
| 🚧 小米 13 Pro | `nuwa` | OS3.0.312.0.WMBCNXM（Android 16） |
| 🚧 小米 14 | `houji` | OS3.0.307.0.WNCCNXM（Android 16） |
| 🚧 小米 14 Pro | `shennong` | OS3.0.309.0.WNBCNXM（Android 16） |
| 🚧 小米 14 Ultra | `aurora` | OS3.0.308.0.WNACNXM（Android 16） |
| 🚧 小米 15 | `dada` | OS3.0.307.0.WOCCNXM（Android 16） |
| 🚧 小米 15 Pro | `haotian` | OS3.0.309.0.WOBCNXM（Android 16） |
| 🚧 小米 15 Ultra | `xuanyuan` | OS3.0.314.0.WOACNXM（Android 16） |
| 🚧 小米 17 | `pudding` | OS4.0.4.0.XPCCNXM（Android 17） |
| 🚧 小米 17 Pro | `pandora` | OS4.0.4.0.XBLCNXM（Android 17） |
| 🚧 小米 17 Pro Max | `popsicle` | OS4.0.4.0.XPBCNXM（Android 17） |
| 🚧 小米 17 Max | `byron` | OS3.0.310.0.WAFCNXM（Android 16） |
| 🚧 REDMI K80 | `zorn` | OS3.0.309.0.WOKCNXM（Android 16） |
| 🚧 REDMI K80 Pro | `miro` | OS3.0.305.0.WOMCNXM（Android 16） |
| 🚧 REDMI K90 | `annibale` | OS3.0.307.0.WPKCNXM（Android 16） |
| 🚧 REDMI K90 Pro Max | `myron` | OS4.0.4.0.XPMCNXM（Android 17） |
| 🚧 REDMI K90 Ultra | `warsaw` | OS3.0.309.0.WHPCNXM（Android 16） |
| 🚧 REDMI K100 Pro | `athens` | OS4.0.4.0.XPICNXM（Android 17） |
| 🚧 REDMI K100 Pro Max | `songyuan` | OS4.0.4.0.XGNCNXM（Android 17） |

## 安装 Skyward

Skyward 需要已解锁的 bootloader。本分支不发布 Releases，镜像按[构建](#构建)中的步骤构建。上游的 zip 在它的 [Releases](https://github.com/kirklin/skyward/releases) 中，每个文件都标有机型和代号，例如 Xiaomi 17 Ultra (nezha)。使用 `SHA256SUMS` 核对并解压之后，在手机处于 fastboot 模式的时候，把其中的镜像刷入当前运行槽位的 recovery 分区：

```sh
unzip skyward-v0.2.0-nezha-OS4.0.4.0.XPACNXM.zip
fastboot flash recovery skyward-v0.2.0-nezha-OS4.0.4.0.XPACNXM.img
fastboot reboot recovery
```

之后连接 WLAN，就可以在“设置”的“Skyward 更新”页面更新到最新版本，也可以在 Skyward 自己的“安装”页面安装解压出来的新版镜像。打开 **自动重新刷入 Skyward** 之后，系统更新或者刷机包覆盖 recovery 分区的时候，Skyward 会重新刷回这些分区。

Skyward 中的 adb 是 root shell，手机上无需授权：手机处于 recovery 的时候，任何人拿到手机和数据线都能访问其中的存储。

## 双系统的工作方式

Skyward 可以在手机自身分区中的系统旁边安装第二系统，主系统是 HyperOS 或者第三方 ROM 都可以，并在首页的“双系统”中切换。第二系统的根文件系统是 data 分区中的一个文件，文件系统不能移动或删除它，Skyward 直接通过分区读写其中的内容；两个系统的启动镜像保存在另一个这样的文件中。切换系统只改写下次启动槽位的 boot 和 init_boot 分区，每一步都先记录在 /metadata 中，切换过程中断电的话，Skyward 下次启动时会完成这次切换。vendor_boot、dtbo、recovery、vbmeta、固件和 super 都保持不变，第二系统使用主系统的 vendor_boot 和 dtbo 启动。

其他操作写入启动分区之前，Skyward 会先把主系统的启动镜像写回：安装、切换启动槽位、fastboot、恢复出厂设置，以及备份或恢复启动分区。系统更新尚未合并的时候，暂不切换到第二系统；备份 data 分区的时候不包含第二系统。

第二系统的安装包是 zip 文件，和其他安装包一样在“安装”页面安装：

| 文件 | 内容 |
| --- | --- |
| `system.toml` | `id`、`name`、`version` 和 `filesystem`；`init` 是根文件系统中第一个运行的程序，默认为 `/sbin/init`；`minimum` 是根文件系统至少需要的空间，单位为字节 |
| `boot.img` | bootloader 启动的内核 |
| `rootfs.img` | 根文件系统的镜像，格式为 `filesystem` 指定的文件系统 |
| `init_boot.img` | 只有自带第一阶段 ramdisk 的系统需要 |

没有自带 init_boot 镜像的系统，从 Skyward 生成的 init_boot 镜像启动。其中的程序按照主系统第一阶段的方式加载第一阶段的内核模块，包括模块之间的软依赖和内核命令行中给模块的参数，然后在 data 分区中找到根文件系统，挂载之后运行其中的 `init`。启动过程中出现任何错误，这个程序都会进入 Skyward，和主系统启动失败时进入 recovery 一样：它在 misc 中留下原因，把当前槽位 recovery 分区中的 ramdisk 按照 bootloader 启动 recovery 时内核解包的方式解包到自己的 ramdisk 上，然后交给 Skyward 的第一个程序运行，所以无论 bootloader 怎样处理重启，都能进入 Skyward。Skyward 显示原因；用户在“双系统”中切换之前，下次启动的仍是第二系统。启动长时间没有进展时，这个程序改为重启到 recovery；交给第二系统之前手机重启的话，下次启动时它会在 misc 中看到同样的请求，先进入 Skyward。两种方式都无法进入 Skyward 时，手机停留在出错的地方，不会反复启动同一个系统。自带第一阶段 ramdisk 的系统可以运行 Skyward 放在 `/skyward/map` 的程序，它把根文件系统映射为块设备，并输出设备节点的路径。每次切换到第二系统，Skyward 都用自己带的程序写入第二系统的 init_boot 镜像，所以更新 Skyward 以后，第二系统也用新的程序启动。

正在运行的系统也可以请求切换：在 misc 中写入 `boot-recovery` 和 `--switch_system=installed`，或者 `--switch_system=` 加上第二系统的 `id`，然后重启到 recovery。bootloader 仍然启动这个槽位的话（小米 13 Ultra 连着 USB 线时可能这样），Skyward 的程序会在下次启动时在 misc 中看到这个请求，先进入 Skyward。

## 构建

Skyward 使用 `rust-toolchain.toml` 指定的稳定版 Rust 和它的 `aarch64-linux-android` 目标、Android NDK r29 和 [cargo-ndk](https://github.com/bbqsrc/cargo-ndk) 构建。recovery 服务针对手机构建，打包工具针对电脑构建：

```sh
cargo ndk -t arm64-v8a --platform 30 build --release -p skyward
cargo build --release -p skyward-pack
```

第二系统启动时运行的第一个程序在任何动态库可用之前运行，因此构建为静态程序：

```sh
RUSTFLAGS="-C target-feature=+crt-static" cargo ndk -t arm64-v8a --platform 30 build --release -p skyward-system-init --target-dir target/system-init
```

打包工具从小米服务器上这个版本的完整更新包中取出原厂 recovery 分区，只读取这个分区所在的部分，并且按照设备配置文件核对 SHA-256：

```sh
target/release/skyward-pack stock --config devices/nezha.toml --out target/stock/nezha/recovery.img
```

然后在它的基础上构建镜像：

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

### 本分支的镜像（`marble`）

原厂 recovery 没有 WLAN 需要的平台库，所以打包 marble 的镜像时用
`devices/marble-extra` 把它们加进 ramdisk，其余步骤与上面相同。

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

刷入手机启动槽位的 recovery 分区：

```sh
fastboot flash recovery_a target/skyward-marble.img
fastboot reboot recovery
```

测试和代码检查在电脑上运行；只在手机上运行的代码按照手机目标检查：

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo ndk -t arm64-v8a --platform 30 clippy -p skyward -p skyward-install -p skyward-systems -p skyward-system-init -- -D warnings
```

`skyward-preview` 无需手机，在电脑上把页面绘制成 PNG 文件：每个页面完整绘制成长截图的样子，覆盖它可能处于的各种状态，另外还有触摸和动画过程中某些时刻的屏幕。`--lang` 按照地区代码选择语言，例如 `en` 或者 `zh-TW`：

```sh
cargo run --release -p skyward-preview -- target/preview --lang en
```

## 许可证

[GPL-3.0-or-later](LICENSE)
