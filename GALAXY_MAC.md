# Galaxy宝可梦 Mac 版

Mac 版和 Windows 版是同一份源码。Mac 专用的行为都用 `cfg!(target_os = "macos")` 分开：Windows 编译时这些代码也会做类型检查，但 Windows 版的行为不变。
Mac 版只能在 Mac 上编译（Tauri/Rust 不能在 Windows 上交叉编译 Mac 程序）。

## 和 Windows 版的区别（2026-11-10）

| | Windows | Mac |
|---|---|---|
| 游戏目录 | 启动器旁边的 `.minecraft` | `~/Library/Application Support/GalaxyPokemon/.minecraft`（文件夹名用英文，部分模组遇到中文路径会出错） |
| 整合包从哪来 | 直装包里自带 | 安装包里自带（`Galaxy宝可梦.app/Contents/Resources/.minecraft`），第一次打开时复制到游戏目录；同一块 APFS 磁盘上是克隆，很快 |
| Java 21 | 整合包自带 `runtime/java-runtime-delta` | 不带；启动器自动下载 Mac 版 Java |
| Mac 本地库（LWJGL 等） | 整合包自带 | 启动前按版本 json 自动补全 |
| 整合包更新 | galaxy_sync | galaxy_sync（同一条通道） |
| 启动器自更新 | galaxy_sync 同步的 launcher.zip | 关闭（那个包是 Windows 版）；新版启动器要重新下载 dmg |
| `galaxy-launcher.json` | exe 旁边 | `Contents/Resources/`（签名时 `Contents/MacOS` 里只能放程序） |

改动的代码：

- `packages/app-lib/src/api/galaxy.rs`：`find_game_root` 在 Mac 上查 `mac_game_root`；`bootstrap` 先调 `install_bundled_game`，把自带整合包装到游戏目录（先复制到 `.minecraft.partial`，完成后改名，中断了下次重来）。
- `packages/app-lib/src/api/galaxy_update.rs`：`launcher_dir` 在 Mac 上指向 `Contents/Resources`；`pending` 只在 Windows 上提供自更新。

## 打包（在 Mac 上）

准备（只需一次）：

1. Xcode 命令行工具：`xcode-select --install`
2. Rust：https://rustup.rs （源码里的 `rust-toolchain.toml` 会自动装对版本）
3. Node.js 20 以上，然后 `corepack enable` 启用 pnpm
4. JDK 21（编译时要用，例如 Adoptium Temurin 21 的 pkg 安装包）

先在 Windows 上运行 `galaxy-pack-mac-kit.ps1`，生成 `dist-mac-kit\GalaxyPokemon-mac-kit.zip`（源码 + 去掉 Windows Java 的整合包）。拷到 Mac 上解压，在 `source` 目录里：

    bash galaxy-build-mac.sh ../mac-client/.minecraft 1.0.7

产物：`dist-mac/GalaxyPokemon_v1.0.7_mac.dmg`，Apple 芯片和 Intel 通用。

脚本做的事：装 Rust 的两个 Mac 目标 → `pnpm install` → `tauri build --target universal-apple-darwin --bundles app` → 把整合包和 `galaxy-launcher.json` 放进 .app → ad-hoc 签名 → 打成 dmg（里面有「应用程序」快捷方式和「打不开时看这里.txt」）。
数据接口地址默认 `http://mc9.yunmaoc.com:28095`，要换就先 `export GALAXY_API_BASE=...`。

## 玩家怎么装

见 `GALAXY_MAC_PLAYERS.txt`（作为「打不开时看这里.txt」放进 dmg）。没有苹果开发者证书，第一次打开会被系统拦住，要在「系统设置 → 隐私与安全性」里点「仍要打开」。

## 还没在真机上验证的

- 编译：上游 macOS 相关代码在瘦身后能否编译，要在 Mac 上第一次编译时确认；报错就把终端输出发回来。
- 第一次启动：Java 和 Mac 本地库要联网下载；整合包里的模组在 Mac 上是否都能用。
- 默认内存：8 GB 内存的 Mac 可能要调小。
