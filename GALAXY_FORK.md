# Galaxy宝可梦 启动器（Axolotl Launcher 分支）

服主决定：启动器直接基于 Axolotl Launcher（美西螈启动器，GPL-3.0-only）源码改，界面保持美西螈原样，
**内容只为 Galaxy宝可梦 一个服务器服务**。本分支同样以 GPL-3.0-only 发布，修改后的源码需要公开。

## 目标

1. **品牌**
   - 应用名改为「Galaxy宝可梦」，图标用 `D:\GalaxyPokemon\branding\icon.svg` / `icon-1024.png`。
   - 窗口标题、关于页、安装包名都换掉。
   - 保留对 Axolotl Launcher 和 Modrinth App 上游的署名与 GPL 声明，但不冒用美西螈的品牌。
2. **只保留一个固定实例**
   - 实例名：Galaxy宝可梦；版本：Minecraft 1.21.1 + Fabric Loader 0.19.5，对应版本 json `1.21.1-Fabric 0.19.3`。
   - 游戏目录：安装目录下的 `.minecraft`，版本隔离，实例目录为 `.minecraft\versions\1.21.1-Fabric 0.19.3`。
   - 首次启动自动关联（Axolotl 支持关联 PCL/HMCL 的现有 .minecraft），不让玩家新建、导入或删除实例。
3. **去掉与本服无关的功能**：Modrinth/CurseForge 内容浏览与安装（Discover/Browse）、整合包导出、实例创建/导入、
   实验室工具、联机（陶瓦/红石）、Modrinth 账户/通知/广告/新闻等远程服务、Axolotl 自己的公告与自动更新源。
   保留：首页（服务器卡片 + 开始游戏 + 公告）、实例页（模组只读列表 / 截图 / 日志 / 设置）、设置、皮肤（离线皮肤）可选。
4. **账户**：本服 `online-mode=false`，默认且只提供**离线账户**（玩家填游戏名，3–16 位字母数字下划线）。
   UUID 必须与服务端一致：`UUID.nameUUIDFromBytes("OfflinePlayer:" + name)`。
5. **一键进服**：启动时带 quick play multiplayer 参数直连 `mc9.yunmaoc.com:27852`（可在设置里关）。
6. **梦鱼更新器（必须）**（2026-11-02 起由启动器自带更新器取代，不再加载 agent，见文末「Built-in updater (galaxy_sync)」；以下为原来的做法）：启动时若实例目录存在 `.dreamingfish-bootstrap/bootstrap-agent.jar`，追加 JVM 参数
   `-javaagent:<实例目录绝对路径>/.dreamingfish-bootstrap/bootstrap-agent.jar`，且游戏进程工作目录 = 实例目录。
   版本 json 已含该参数时不要重复添加。agent 拒绝放行时（日志含 `DreamingFish updater blocked Minecraft startup`）给出中文提示。
7. **启动器自身更新**：Tauri updater 指向我们自己的地址或关闭；不得继续指向 Axolotl 官方更新源。
   计划沿用梦鱼通道：`<实例>/launcher-update/latest.json` 由梦鱼同步（后续再定实现）。
8. **Java**：优先用 `.minecraft/runtime/java-runtime-delta/bin/javaw.exe`（安装包自带 Java 21）。
9. **服务器状态**：首页显示 `mc9.yunmaoc.com:27852` 在线人数/延迟/MOTD（Axolotl 若有服务器列表 ping 可复用）。

## 不变的约束
- 不碰 `D:\GalaxyPokemon\GalaxyPokemon正式服`（线上服务器）。
- 数据目录用独立标识（不要和正版 Axolotl 共用 `%APPDATA%\red.ghs.axolotl`）。
- 旧的 Electron 启动器 `D:\GalaxyPokemon\GalaxyPokemon-Launcher` 保留：里面有出包脚本（client/、prepare-client.ps1、
  make-package.ps1、make-installer.ps1、dfs-release.ps1）和梦鱼发布流程，新启动器替换掉的只是其中的 launcher 部分。

## Slimming phase 1 (frontend)

依据 `GALAXY_SLIM.md`，只动前端，没有动 Rust。前端 `apps/app-frontend/dist`：884 个文件 / 50.42 MB → 583 个文件 / 15.70 MB
（`_devtools/slim/sizes.log`）。改动前的原文件、移走的目录都在 `D:\GalaxyPokemon\_devtools\slim\phase1-backup\`（按仓库相对路径存放），
恢复方法：把对应文件或目录拷回仓库原位置。

- `apps/app-frontend/public/` 下的 `monaco/`（Monaco 编辑器 + worker，23.4 MB）、`easteregg/`（6.3 MB）、`seed-map-assets/`（2.5 MB）、
  `models/`（关于页美西螈模型）已移走，`public/` 现为空目录。仍引用这些 URL 的源码（`StudioEditor.vue`、`EasterEgg*Modal.vue`、
  `AboutScene.vue`、`lab/seed-map/*`、`SeedMapBiomePicker.vue`）都不在构建里（没有注册的路由，也没有被引用）；皮肤预览用的是
  `packages/assets/models/*.gltf`，不受影响。
- `components/ui/settings/settings-registry.ts`：已隐藏的设置分区（主页与导航、AI、MCP、实例同步、内容与下载、联机、更新）
  改用空的占位组件，不再打包 UpdateSettings（含 5,900 行的 Axolotl 更新公告 catalog）、AISettings、McpSettings、
  instances/（含 Monaco 命令编辑器）等。文件内注释写了每个分区原来对应的页面。
- `App.vue`：不再挂载 CreationFlowModal、UnknownPackWarningModal、ContentInstallModal、ContentInstallPreviewModal、
  ResourceOperationPreviewModal、ModpackAlreadyInstalledModal、AddServerToInstanceModal、ContentUpdaterModal、
  ModpackInstallModal、CurseForgeManualDownloadsModal、拖放/处理中遮罩、ConfirmDropTypeModal、GenericContentInstallModal、
  InstanceExportModal、LauncherImportModal、SymlinkMethodCards、兼容模式导入弹窗、BatchScanOverlay、McpConnectionOverlay；
  去掉 `useDropImport` 整套拖放导入、`startMcpStateSync` / `startMcpUiBridge`、内容选择 provider、启动时配置 CurseForge
  手动下载监视器（会监视「下载」文件夹）。深链处理保留：`OpenRoute` / `OpenSettings` / `UpdateSettings` / `StopInstance` /
  `LaunchInstance` 照旧；`RunMRPack`（打开 .mrpack）、`InstallVersion` / `InstallMod` / `InstallModpack` / `InstallServer`
  改为「Unsupported link」提示。保留的：InstanceIconPickerModal（实例设置 → 常规 → 图标会用到）、内容安装 / 服务器安装
  provider（实例页 Mods.vue、instance/Index.vue 会 inject）、崩溃 / 错误 / Java 下载确认 / 关闭方式 / 备份退出 / 启动器更新弹窗、
  通知、新手引导、I18nDebugPanel（开发者模式的功能开关）。皮肤页和外观设置的拖放有各自的处理，不受影响。
  改动由 `_devtools/slim/edit-app-vue.mjs` 生成，可对 phase1-backup 里的原文件重放。
- `providers/setup.ts`：不再安装 creation-modal、file-drop、instance-import 三个 provider（文件还在 `providers/setup/`）。
- `composables/useLogAgent.ts`：本地 LogAgent（deepagents + langchain，约 2.2 MB JS）改为直接报错，崩溃弹窗的云端
  LogShare 分析不变。本地分析需要在已隐藏的 AI 设置里配置模型，所以本来就用不到。原来的一行写在注释里。
- `components/ui/AppTitlebar.vue`：去掉 McpConnectionStatus；`AppTitlebar.visual.spec.ts` 的 MCP 用例改为断言没有这个控件。
- 验证：前端 build 通过；`vue-tsc --noEmit -p tsconfig.app.json` 改动前后对比没有新增错误（注意：package 里的
  `vue-tsc --noEmit` 用的是 `files: []` 的 tsconfig.json，实际不做类型检查）；brand-guard、i18n-check 通过；harness 截图
  `_devtools/v5-*.png` 与改动前的构建逐字节相同（设置页肉眼一致），控制台错误与红色提示和改动前一样。
- 留给以后：其余 ~25 种界面语言（约 2.4 MB 按需加载的 chunk）和 zh-TW 仍在语言列表里，没删；翻译设置里的 AI 模型选项
  会按需加载 332 个 lobehub 图标 chunk（0.83 MB）；开发者模式功能开关页的 McpDebugSettings 仍会调用 `plugin:mcp|*`。

## Slimming phase 2 (Rust)

依据 `GALAXY_SLIM.md` 第 2 阶段，只动了 `apps/app`（Tauri 外壳）、一个前端文件和 `galaxy-stage.ps1`，`packages/app-lib` 没有改。
决策表在 `D:\GalaxyPokemon\_devtools\slim\phase2-plan.md`，依据是前端构建产物里出现的 `plugin:<名字>|<命令>` 字符串
（`_devtools/slim/ipc-usage.mjs`）。改动前的原文件都在 `D:\GalaxyPokemon\_devtools\slim\phase2-backup\`（按仓库相对路径存放），
上一版 `dist-launcher` 也整份备份在那里。

结果（`_devtools/slim/sizes.log`，`phase2-rust`，release、opt-level "s"、无 LTO，含第 1 阶段的前端瘦身）：
`GalaxyPokemon.exe` 184,960,000 → 143,823,872 字节（176.39 → 137.16 MB）；`dist-launcher` 8 个文件 / 178.80 MB → 5 个文件 / 137.20 MB；
自更新 zip 76.25 → 53.15 MB。按第 1 阶段的 brotli 估算，嵌入的前端少了约 12.4 MB，其余约 28.7 MB 来自本阶段。

- **不再注册的 Tauri 插件**：`main.rs` 的 `.plugin(...)`、`src/api/mod.rs` 的 `pub mod`、`build.rs` 的 `InlinedPlugin`、
  `capabilities/plugins.json` 的 `<名字>:default` 一起去掉；源码文件都还在磁盘上，只是不再编译。
  - `mcp`（MCP 服务器，原本默认关）：连带 `api/tools` 运行时、`initialize_state` 里的 `api::mcp::initialize`、退出时的 `api::mcp::shutdown`。
  - `seed-map`（连带 `mod seed_map` 和它 include_bytes 的资源）、`schematic-preview`、`mod-translation`（连带 `mod mod_translation`）。
  - `terracotta`、`multiplayer`（陶瓦 / 红石联机隧道，只由它们自己的命令启动）、`servers`（本地开服）、`friends`、`import`（其他启动器导入）、
    `drop`（拖放导入识别）、`metadata`、`mcarchive`、`planet-minecraft`、`content-favorites`。
  - 退出时的 `theseus::multiplayer::shutdown()`、`servers::shutdown_all()` 和轻量模式里的 `multiplayer::stop()` 保留，没有隧道或服务器在跑时它们什么也不做。
- **收窄的插件**：`ai` 只注册 `ai_get_catalog`、`ai_get_state`（崩溃弹窗和翻译设置在用），去掉了 `ai_update_settings`、`ai_update_provider`、
  `ai_set_api_key`、`ai_set_credential`、`ai_update_model`、`ai_remove_model`、`ai_fetch_models`、`ai_test_provider`、`ai_begin_oauth`、
  `ai_poll_oauth`、`ai_disconnect_oauth`。`install` 去掉 `install_get_modpack_preview`、`install_import_instance`、`install_start_import_plan`、
  `install_cancel_import_plan`，`install_existing_instance` 和安装 / 下载任务命令都在。`instance`、`curseforge`、`cache`、`files` 等插件没动。
- **Blockbench 皮肤编辑器和种子地图引擎**：`/lab/skin-editor` 路由没有注册，所以去掉了 `axolotl-skin://` 协议处理和 `build.rs` 里的 Blockbench 构建步骤；
  `build.rs` 里 cubiomes 的 MinGW 编译步骤也去掉了，`Cargo.toml` 去掉 `cc` 构建依赖。`tauri.conf.json` / `tauri.windows.conf.json` 的
  `bundle.resources` 改为 `[]`（原来是 `resources/blockbench-skin`、`resources/seed-map`），`apps/app/resources/` 下的文件留在原处但不再打包。
  `galaxy-stage.ps1` 改成 `target\release\resources` 存在时才复制，所以 `dist-launcher` 里不再有 `resources\`（原来的 2.5 MB DLL）。
  `galaxy-env.ps1` 里的 `AXOLOTL_CUBIOMES_CC`、`AXOLOTL_SKIP_BLOCKBENCH_BUILD` 现在没人读，留着无害。
- **前端**：`components/ui/settings/FeatureFlagSettings.vue`（开发者模式的功能开关页）不再挂载 `McpDebugSettings`，构建产物里已没有 `plugin:mcp|`。
- **Cargo 依赖**：去掉了不再使用的 `base64`、`bytes`、`cap-fs-ext`、`cap-std`、`daedalus`、`flate2`、`quartz_nbt`、`regex`、`tempfile`、`zip` 和
  构建依赖 `cc`（`cargo tree -e features` 对比，共享依赖的 feature 不变）。`launcher-mcp`、`launcher-tools`、`libloading` 也没人引用了，代码不会被链接，
  但删掉它们会改变 hyper-util、tower、axum、winapi 的 feature 组合，app-lib 就得整个重编；这台机器在两个服务器都开着时内存不够，
  所以先留着（`Cargo.toml` 里有注释），等哪次本来就要全量重编时再删。
- **后台任务**：app-lib 的 `State::init` 没动。内容文件恢复、安装任务恢复、config_sync、下载并发控制、遥测循环（没有上报地址）、
  Google 翻译 IP 预热都服务于保留的功能。不再启动的只有 MCP 服务。
- **构建**：本阶段只重编了 `theseus_gui`。它的 release 编译峰值提交内存约 6.0 GB（工作集约 3.0 GB），在两个服务器都开着、只剩约 7.3 GB 可用内存时，
  `CARGO_BUILD_JOBS=3` 和 `=1` 都会被看门狗在可用内存跌破 3 GB 时杀掉；最后用 `CARGO_BUILD_JOBS=1` 加看门狗的“内存不够时把 rustc 的工作集换出到页面文件”
  （`_devtools/slim/run-build.ps1 -TrimBelowMB 4200`）在 13.6 分钟内编完，期间可用内存最低 3.98 GB。
- **验证**：`cargo check -p theseus_gui --release` 通过（只剩原有的 `curseforge.rs` 未用 import 警告）；前端构建通过；brand-guard 通过；
  release exe 冒烟 25 秒：窗口标题 Galaxy宝可梦，日志有 `Linking the Galaxy宝可梦 game directory`，ERROR 只有原来就有的两条
  “User is not logged in”，没有 panic；harness 截图 `_devtools/v6-*.png` 与 v5 对比：7 张逐字节相同，其余 3 张看上去一致，
  控制台错误、提示和 IPC 调用列表都和 v5 相同。
- **恢复某个插件**：把 `phase2-backup` 里 `main.rs`、`api/mod.rs`、`build.rs`、`capabilities/plugins.json`（`ai`、`install` 还有 `api/ai.rs`、`api/install.rs`）
  中对应的几行拷回来，再 `cargo check -p theseus_gui --release`。种子地图另外要恢复 `build.rs` 的 cubiomes 步骤、`cc` 构建依赖和
  `tauri.windows.conf.json` 的 resources；皮肤编辑器要恢复 `main.rs` 的 `axolotl-skin` 协议、`build.rs` 的 Blockbench 步骤和两个 conf 的 resources；
  MCP 还要把 `McpDebugSettings` 放回 `FeatureFlagSettings.vue`。

## 启动超时放宽（1.0.4，2026-11-02）

梦鱼更新器是 `-javaagent`，在游戏进程的 premain 里先检查、下载整合包，之后游戏才连回启动器。上游的限制是
游戏 15 秒内必须连回（`state/process.rs` 的 `PROCESS_INITIALIZATION_TIMEOUT`），整个启动准备 60 秒
（`api/instance/run.rs`），第一次启动或大更新时梦鱼还没下完，启动器就会把游戏杀掉。

- `api/galaxy.rs`：`bootstrap` 记下带梦鱼 agent 的实例 id；`launch_timeout_for` 对这个实例把超时提到至少
  `DREAMINGFISH_LAUNCH_TIMEOUT_SECS`（30 分钟），其他实例不变。
- `api/instance/run.rs`：启动准备超时经过 `launch_timeout_for`。
- `state/process.rs`：「连回启动器」的等待改为 max(15 秒, 启动准备超时)，报错文字里的秒数跟着变。
- 前端：首页启动中超过 8 秒、且有梦鱼时，开始栏显示「梦鱼正在检查整合包更新，第一次启动可能要几分钟，请稍等」；
  崩溃弹窗的两条超时文字去掉了写死的 15 / 60 秒，并提示可能是梦鱼还在下载。
- 改动前的文件在 `D:\GalaxyPokemon\_devtools\launch-timeout-backup\`。

## Built-in updater (galaxy_sync)（2026-11-02）

依据 `GALAXY_SYNC.md`。启动器自己就是梦鱼更新服务器的客户端：打开启动器时检查整合包更新，有更新就自动下载、校验、安装，
进度显示在首页底栏，不再有游戏内的梦鱼窗口，也不再加载 `-javaagent`。服务器端完全不变（DreamingFishAdmin `serve`，
公网 `http://mc9.yunmaoc.com:28080`，用 `GalaxyPokemon-Launcher\scripts\dfs-release.ps1` 发布）。取代了上面第 6 条和
「启动超时放宽（1.0.4）」里的超时部分。改动前的文件都在 `D:\GalaxyPokemon\_devtools\sync-backup\`（按 `D:\GalaxyPokemon\` 下的相对路径存放）。

### 代码
- `packages/app-lib/src/api/galaxy_sync.rs`：绑定 / 网络 / 状态文件 / 进程内锁 / 进度事件 / `check`、`apply`、`status`。
- `packages/app-lib/src/api/galaxy_sync/engine.rs`：清单解析和校验、计划规则、哈希缓存、安装与回滚、备份清理。它不用网络、
  也不引用 crate 里的其他模块，所以能单独编译测试：`_devtools\sync\engine-test`（`cargo test --offline`，15 个规则用例 +
  `tests/real_data.rs` 用真实 r7 基线验签）。在 app-lib 里同样是 `#[cfg(test)]` 用例。
- 签名用 `ed25519-dalek 2.2`（`verify_strict`，直接校验收到的原始字节，从不重新序列化）。它本来就因 jsonwebtoken 的
  `rust_crypto` 后端在依赖图里，特性组合不变，不用下载新 crate；工作区 `Cargo.toml` 加了 workspace 依赖，`Cargo.lock` 只多了
  theseus 对它的一条依赖。
- 命令：`apps/app/src/api/galaxy.rs` + `apps/app/build.rs`（`galaxy_sync_check` / `galaxy_sync_apply` / `galaxy_sync_status`），
  事件 `galaxy_sync_progress`（`event/emit.rs`）。接口写在 `GALAXY_CONTRACT.md` 末尾。

### 信任和检查
- 绑定：`<实例>\.dreamingfish-bootstrap\project-binding.json` 的 `baseUrl` / `projectId` / `publicKey`。文件不存在、读不出或
  某一项无效时用内置常量（`DEFAULT_BASE_URL` = `http://mc9.yunmaoc.com:28080`、`galaxy-pokemon`、r7 绑定里的公钥）。
  公钥接受 44 字节 X.509 SPKI DER（前缀 `302a300506032b6570032100`）或 32 字节原始公钥。
- `GET {baseUrl}/v1/projects/{id}/latest`，`Accept: application/json`、`Accept-Encoding: identity`，用启动器配置的 HTTP 客户端
  （代理设置生效），连接 + 响应头 10 秒、整体 60 秒，最多 32 MiB；签名取 `X-Dfs-Signature`，没有就取 `{uri}.sig`。
  429 / 503 按 `Retry-After` 重试两次。连不上、超时、非 2xx → `offline`；签名不对 → `blocked`（`SIGNATURE_INVALID`）。
- 校验：`schemaVersion` 1、`projectId` 一致、`requiredCapabilities` 只能是 `forced-directory-sync-v1`、`forced-file-sync-v1`、
  `released-paths-v1`、`maintenance-policy-v2`、`simplified-maintenance-v1`，`minimumPlayerVersion` ≤ 0.3.2（按数字比较），
  不满足 → `blocked`（`NEEDS_NEWER_LAUNCHER` / `PROJECT_MISMATCH`）。
- 防回放：`sequence` 必须 ≥ 本机见过的最高序号，序号相同时清单 SHA-256 也必须相同（`REPLAY`）。最高序号取 state.json 的记录和
  内置基线（`bundled-release\manifest.json` + `.sig`，也会验签，签名不对就忽略）中较大的一个，所以第一次打开就以安装包里的 r7 为底。
- 本地记录：每次检查都会用同一把公钥验 `.galaxy-sync\release-manifest.json`。被改过（签名不对、内容也和 state.json 记录的
  SHA-256 不同）→ `blocked`（`LOCAL_STATE_INVALID`，提示点「修复游戏文件」）；「修复游戏文件」时丢掉旧记录（保留
  initializedPaths 和已执行的修正）重新开始。只是公钥换了（内容和记录一致、签名不对）→ 记一条警告，重新以基线开始。

### 计划规则（`engine::validate` + `engine::build_plan`）
| 规则 | 实现 |
|---|---|
| 路径安全 | `safe_relative_path`：拒绝空路径、`/` 开头、`\`、`:`（盘符和 NTFS 流）、`.` / `..` / 空段、Windows 不能存的字符、以点或空格结尾的段、设备名（CON、NUL、COM1…）以及 `.galaxy-sync` / `.dreamingfish-bootstrap` 里的路径 → `blocked`（`PATH_UNSAFE`）。文件、撤回项、修正、目录列表全部检查。Windows 上比较时不区分大小写（`path_key`），清单里大小写不同的重复路径算无效清单。 |
| 可选分组 | `optionalGroup` 属于 `defaultInstall:false` 的分组 → 当作清单里没有这个文件（之前装过且没改过的会按「已移除」处理）。 |
| SYNC / REQUIRED / forcedSyncFiles | 本地缺失 → 下载；大小或 SHA-256 不同 → 原文件进备份，再装新的。 |
| INITIAL / DEFAULT_CONFIG / LEGACY_MISSING_ONLY | 已在 `initializedPaths` → 不再碰（删了也不补）；本地已有 → 保留并记入；缺失 → 下载并记入。第一次打开时安装包自带的 options.txt / servers.dat 只会被记下。 |
| 上个版本有、这个版本没有 | `state.files` 里的路径不在新清单 → 本地文件还是旧 SHA-256 才进备份，玩家改过的不动。 |
| withdrawals（REMOVAL / VERSION） | 路径不是当前清单文件、本地大小和 SHA-256 都与撤回项一致 → 进备份（每次检查都执行，装回来也会再被移走）。 |
| corrections KNOWN_BAD | 本地 SHA-256 在 `badSha256` 里 → 进备份；是清单文件就再装正确的（INITIAL 文件也一样）。 |
| corrections ONCE | id 不在 `appliedCorrections` 时执行一次：清单文件与发布版不同 → 备份后重装（缺失则安装）；不是清单文件 → 备份。执行后记 id。 |
| forcedSyncDirectories / cleanupDirectories | 目录下不属于清单的文件进备份（不跟随符号链接 / 目录联接）。当前发布里都是空的，按规则实现。 |
| releasedPaths / retainedSelfManagedPaths | 这些路径（含子路径）完全不管：不装、不删、不清理。 |
| 其他 | 存档、截图、日志、玩家自己加的模组（不在清理目录里）、`.galaxy-sync`、`.dreamingfish-bootstrap` 都不碰。不做重复 modid 检测。 |

### 状态目录 `<实例>\.galaxy-sync\`
- `state.json`：`{schemaVersion:1, projectId, releaseId, sequence, manifestSha256, highestSequence, highestManifestSha256,
  displayVersion, files:[{path,sha256,size}], initializedPaths, appliedCorrections, syncedAt}`，最后一次完整应用的版本。
- `release-manifest.json` + `release-manifest.sig`：应用的签名清单原样保存。
- `file-index.json`：`{<小写路径>: {size, mtimeMs, sha256}}`。大小或修改时间变了才重新算哈希；「修复游戏文件」全部重算。
  第一次检查约 1186 个文件 / 1.3 GB，4 个线程。
- `downloads\<sha>.part`：断点续传（`Range: bytes=N-`，校验 206 的 `Content-Range`；416 / 位置不对就从头下），
  大小和 SHA-256 都对才算下完，不对就删掉重下一次，再不对就失败；最多同时 4 个。成功后清空。
- `backups\<yyyyMMdd-HHmmss>_<releaseId>\`：被替换 / 移除的文件按原路径存放，附 `index.json` `{releaseId, createdAt, items:[{path, reason}]}`。
  每次成功后只留最新 3 个。
- 所有状态文件先写 `.tmp` 再改名。

### 应用（`apply`）
- 游戏在运行时拒绝（「请先关闭游戏，再更新整合包」）；检查 / 应用 / 启动共用一把进程内锁（`try_launch_guard`）。
- 先检查（复用 5 分钟内验过的清单），再确认磁盘空间（需要下载量 + 64 MB），全部下载并校验完才开始动文件：先把要替换 / 删除的
  文件移进备份目录，再把下载好的文件改名到位（同一磁盘内改名；跨盘时复制 + 删除）。任何一步失败都按相反顺序撤销
  （装上的文件放回 downloads 供重试，备份移回原处），返回错误，不会留下半个 mods 文件夹。最后写 state.json、清单副本、哈希缓存，清理旧备份。
- 改了 `launcher-update/*` 时结果带 `launcherUpdateChanged`，前端马上重新检查启动器更新，「启动器有新版本」弹窗会立刻出现。

### 启动
- `launcher/mod.rs`：不再加 `-javaagent`；版本 json 或自定义 JVM 参数里带的梦鱼 agent 会被去掉（`strip_dreamingfish_agent`，
  日志 `Removed the DreamingFish -javaagent from the JVM arguments`）。
- 去掉了 `DREAMINGFISH_LAUNCH_TIMEOUT_SECS` / `launch_timeout_for` 和 bootstrap 里记实例 id 的槽，启动准备超时恢复上游默认；
  `state/process.rs` 的「连回启动器」等待 = max(15 秒, 启动准备超时) 保留。
- `GalaxyBootstrap.dreamingfish`：找到游戏就是 true（内置常量兜底）。`galaxy_launch_blocked` 事件和 App.vue 的提示保留，不会再触发。

### 前端
- `store/galaxy.ts`：`bootstrap()` 找到游戏后先看 `galaxy_sync_status`（页面重载时跟住正在进行的同步，5 分钟内的「已是最新」直接用），
  否则 `sync()`：检查，有更新且游戏没在运行就自动 `apply`。窗口重新获得焦点、距上次检查超过 30 分钟时再查一次；游戏运行时发现的
  更新在游戏退出后自动安装。
- 首页底栏（`pages/Index.vue`）：检查中「正在检查整合包更新…」；首次哈希「首次检查整合包文件 n/N…」（修复时「正在校验…」）+ 进度条；
  下载「正在更新整合包 12/40 · 35/120 MB」+ 进度条；「正在安装更新…」；「整合包已是最新 · 1.0.5」（点开看更新说明和 /history 历史版本）；
  离线「暂时连不上更新服务器，将使用本地整合包」+ 重试（可以开始游戏）；blocked / error 红字原因 + 重试，开始游戏禁用。
  同步进行时按钮显示「更新中…」；点开始游戏会先检查并安装更新再启动。装完更新弹「整合包已更新到 x」提示。
- 「修复游戏文件」：原来的 Minecraft / 加载器修复之后再 `sync({ force: true })`（全部重新哈希）。
- 去掉 / 改写的梦鱼文案：首页「梦鱼自动更新已启用」「梦鱼正在检查整合包更新…」、崩溃弹窗超时提示（MinecraftCrashModal、
  zh-CN / en-US）里的「梦鱼还在下载」、修复说明。`galaxy/announcements-bundled.ts` 里没有梦鱼相关条目，没改。
- harness：`_devtools\ui-harness\serve.mjs` 加了 `?sync=checking|hashing|downloading|applying|uptodate|offline|blocked|error|update|applied`
  的模拟（默认 uptodate），`shoot-v7-sync.mjs` / `shoot-v7-toast.mjs` 截图 `_devtools\v7-sync-*.png`。

### 打包
- `scripts\dfs-release.ps1 -Deploy`：只把部署包里的 `.dreamingfish-bootstrap\project-binding.json` 和 `bundled-release\` 放进
  client 的版本文件夹，缺了就报错；不再复制 `bootstrap-agent.jar` 和 `DreamingFishUpdater\`，client 里有就删掉；顺便清掉测试留下的
  `.galaxy-sync`。头部注释改成「玩家下次打开启动器时自动更新」。
- client 文件夹：`DreamingFishUpdater\`（109 个文件 / 64.5 MB）和 `.dreamingfish-bootstrap\bootstrap-agent.jar`
  （4,278,081 字节）已移到 `_devtools\sync-backup\GalaxyPokemon-Launcher\client\...`；保留 `project-binding.json` 和 `bundled-release\`。

### 日志
launcher 日志里每一步都有 `Galaxy sync:` 开头的行：`check started` / `release manifest verified`（release、sequence、字节数、SHA-256）/
`planned`（前 20 个动作）/ `hashed local files` / `downloading` / `files installed` / `removed old backups` /
`check finished` / `apply finished`（status、code、下载数和字节、移除数、哈希数、安装数、备份数、备份目录、耗时、消息）。

### 验证（2026-11-02）
- `cargo check -p theseus_gui --release --offline` 通过（2.2 分钟，提交内存峰值 5.6 GB，新代码没有警告）；前端构建通过；
  `vue-tsc --noEmit -p tsconfig.app.json` 在改过的 TS / Vue 文件里没有错误（其余是原有错误）；engine 单元测试 15 + 1 通过。
- release 构建：`tauri build --config tauri-release.conf.json --no-bundle`，29.1 分钟，`CARGO_BUILD_JOBS=1` + 看门狗换页，
  提交内存峰值 7.9 GB，可用内存最低 3.08 GB，没有被杀。`galaxy-stage.ps1` 之后 `dist-launcher\GalaxyPokemon.exe` 144,366,592 字节（版本仍为 1.0.4）。
- 端到端（不启动游戏）：client 的 `.minecraft` 复制到 `_devtools\sync-test`，`%APPDATA%\com.galaxypokemon.launcher` 先改名放到一边、测完原样
  放回（22 个文件大小 / 时间 / SHA-256 全部一致），用 `GALAXY_GAME_ROOT` 跑 `target\release\GalaxyPokemon.exe`。脚本 `_devtools\sync\e2e-run.ps1`
  （UI Automation 读窗口文字，测试进程加 `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--force-renderer-accessibility`）、`e2e-repair.ps1`、
  `fake-dfs.py`，日志和 UIA 记录在 `_devtools\sync\e2e\`。
  1. 第一次打开：哈希 1183 个文件 / 1.38 GB 约 1 秒（文件都在缓存里），r7（1.0.5）签名通过；state.json 写入，initializedPaths =
     options.txt、servers.dat。下载了 1 个文件（见下面的已知问题）。再次打开：0 个文件需要哈希，约 4 秒显示「整合包已是最新 · 1.0.5」。
  2. 删掉 `mods/ItemPhysicLite_FABRIC_v1.6.13_mc1.21.1.jar`、改坏 `config/terralith.json` → 检测到 2 个文件 / 23,977 字节，从公网服务器下载，
     两个文件的 SHA-256 都回到清单值，改坏的 terralith.json 在 `backups\20261102-160925_r000007-…\config\terralith.json`；弹出「整合包已更新到 1.0.5」。
  3. 改 options.txt → 不覆盖（SHA-256 不变）。4. mods 里加一个 jar → 不动。
  5. binding 的 baseUrl 改成 `http://127.0.0.1:9` → offline，「暂时连不上更新服务器，将使用本地整合包」+ 重试，开始游戏可点。
  6. 改清单副本的 1 个字节 → blocked（LOCAL_STATE_INVALID），开始游戏禁用；点「修复游戏文件」→ 全部重新哈希 1184 个文件，恢复为最新。
     假的 latest（清单改 1 个字节、原签名）→ blocked（SIGNATURE_INVALID），开始游戏禁用。
- harness 截图：`_devtools\v7-sync-{checking,hashing,downloading,applying,uptodate,details,offline,blocked,error,update-playing,applied,applied-toast}.png`。

### 已知问题 / 没做的
- **I18nUpdateMod 会改受管理的文件**：`resourcepacks/Minecraft-Mod-Language-Modpack-Converted-1.21.1.zip` 在发布里是 SYNC，但游戏每次启动时
  I18nUpdateMod 会重写它（client 里这份就是 10-31 测试进游戏时被改的，2,055,485 字节，发布版 2,183,089 字节）。结果是每个新安装第一次打开、以及
  玩家每次玩过之后再打开启动器，都会重新下载这 2.1 MB 并把旧的放进备份。建议在梦鱼管理端把它设成「首次提供」或「停止维护」，或者把 client
  里的这一份换回发布版再出包。原来的梦鱼 agent 也会这样（同样的规则），只是之前没人注意到。
- 本机设置了系统代理 `127.0.0.1:7897`；启动器的「系统代理」模式没有跳过 `127.*`（ProxyOverride），所以 127.0.0.1 上的测试服务器要把测试数据库里的
  代理改成「不用代理」才连得上。对玩家连公网服务器没有影响。
- 测试时 WebView2 的 `%LOCALAPPDATA%\com.galaxypokemon.launcher\EBWebView` 是共用的（之前的冒烟测试也一样），没有隔离。
- 不做：重复 modid 检测；可选分组的玩家开关（只按 `defaultInstall`）；`kind` = VERSION 的撤回只按内容指纹处理，不按模组 ID / 版本识别改名后的副本。
- `GalaxyPokemon-Launcher\README.md` 第 95、101 行还写着旧的 agent 流程，没改（按约定那边只改了 dfs-release.ps1）。

## 名字旁的头像（1.0.6，2026-11-05）

服主在真机上看到右上角、底栏、账号列表的头像都是灰色方块。原因是 `AccountsCard` 拿的是 `batch-skin-renderer` 的 blob: 头像地址，
而那个缓存在渲染完的同一轮微任务里就把「皮肤页没在显示的」头像全部 revoke 了（`pruneSkinPreviewMemory`），图片加载失败，`Avatar` 换成方块。

- `galaxy/player-head.ts`：头像改成 PNG data URL（`getPlayerHeadDataUrl`），不再受那个缓存的回收影响。
- `galaxy/default-skin.ts`：按原版规则算离线玩家的默认皮肤：UUID = `nameUUIDFromBytes("OfflinePlayer:" + 名字)`，
  `DEFAULT_SKINS[floorMod(uuid.hashCode(), 18)]`，前 9 个 slim、后 9 个 wide（对照 1.21.1 客户端 jar 的 `grd` 类核实）。
  和 Python 参考实现、正式服 usercache 的 UUID 对过 8 个名字，全部一致。
- `AccountsCard.vue`：每个已保存的名字显示自己的默认形象；当前账号显示「皮肤」页装备的皮肤。
- `GalaxyPlayerNameModal.vue`：像 PCL 一样，输入名字时头像实时变成这个名字在游戏里的默认形象，下面写「默认形象 / Noor」。
- `packages/app-lib/src/api/minecraft_skins.rs`：没选皮肤时，「已装备」的默认皮肤从固定 Steve 改成按 UUID 算出的那个
  （`vanilla_default_skin_for`），皮肤页、头像和游戏里一致。
- 验证：`_devtools/ui-harness/serve-skins.mjs`（在 serve.mjs 基础上加了皮肤命令的模拟）+ `shoot-heads.mjs`：首页、账号菜单、
  去皮肤页再回来、输入名字四个场景的头像都是能加载的 data:image/png。没有在真机上测（本机没显卡）。
- release 构建 16.7 分钟（app-lib 已缓存）；第一次构建被 MineBackup 的 7za（占了 11 GB）挤爆内存，看门狗杀掉后重跑。
## 首页新功能：新手上路、主服状态、我的数据、活动、画质档位、崩溃上传（1.0.7，2026-11-05）
服务器侧是正式服的新插件 GalaxyLauncherApi（源码 `D:\GalaxyPokemon\_devtools\galaxylauncherapi`，报告
`正式服\_tools\launcherapi_20261105\报告.md`），接口约定 `GALAXY_LAUNCHER_API.md`，命令约定见 GALAXY_CONTRACT.md 最后一节。
- **数据接口地址**：`galaxy-stage.ps1 -ApiBase`（默认 `http://mc9.yunmaoc.com:28095`）写进 `launcher\galaxy-launcher.json` 的 `apiBase`；
  开发时用环境变量 `GALAXY_API_BASE`。`galaxy_bootstrap` 把它带给前端（store `apiBase`）。没有地址或连不上时，下面依赖接口的部分自动隐藏。
  公网端口要服主在服务商那里映射：28095 → 192.168.1.179:8095。
- **主服状态**：代理 `ping-passthrough = "DISABLED"`，原来的 ping 只反映 Velocity。现在在线人数、在线玩家（原版默认头像，按名字算）、
  TPS、维护状态来自 `/v1/summary`，每 30 秒刷新（窗口不可见时暂停）；维护时首页顶部是「维护中」和说明。接口连不上时退回 ping 的数字，
  并提示「主服数据暂时获取不到」。MOTD、图标、延迟仍来自 ping。
- **我的数据**（`GalaxyPlayerStats.vue`）：`/v1/player/<当前名字>`，通宝 / 点券 / 等级 / 通行证 / 签到 / 提醒，只显示有值的格子；
  没进过正式服（`known: false`）就不显示。游戏退出 5 秒后刷新一次。
- **活动与赛季**（`GalaxyEventsList.vue`）：summary 的 `events`，进行中的有倒计时和进度条，没开始的显示「还有 … 开始」，结束的不显示。
- **公告**：`GALAXY_ANNOUNCEMENTS_URL` 为空时改用 `<apiBase>/v1/announcements`（PlayerAnnouncement 的实时列表），缓存和兜底规则不变。
- **新手上路**（`GalaxyOnboardingCard.vue`，状态在 localStorage `galaxy.onboarding.v1`）：起名 → 第一次进服 → 注册登录 → 序章和初始宝可梦。
  前两步启动器自己判断；后两步看接口的 `known`（进过正式服 = 完成），接口不可用时可以手动勾。可以「不再显示」，全部完成后自动隐藏。
- **画质档位**（快捷操作 → `GalaxyGraphicsModal.vue`，后端 `galaxy_graphics.rs`）：低配流畅 / 均衡 / 高画质（开光影，默认
  ComplementaryReimagined）。只改 `options.txt` 的视频项和 `config\iris.properties` 的光影开关，别的行不动；当前设置不属于任何档位时先备份到
  `<实例>\.galaxy-graphics\`，可以「恢复我自己调过的设置」。游戏运行时拒绝。视距上限 10：正式服 `view-distance=8`，再高只掉帧。
- **同步规则改动**：发布里 `config/iris.properties`、`config/sodium-options.json` 是 SYNC，结果玩家在游戏里改的视频设置 / 光影每次启动前都被
  改回去（文件会进备份，提示「整合包已更新」）。现在这两个文件按 INITIAL 处理（`engine.rs` 的 `PLAYER_PREFERENCE_FILES`）；
  服主想强推新版本时仍可用 `forcedSyncFiles` 或 corrections。建议之后在梦鱼管理端也把它们设成「首次提供」。
- **崩溃上传**：崩溃窗口多了「上传给服主」，把这次的崩溃报告 / latest.log 等（最多 4 个）发到 `/v1/crash`，返回报告编号给玩家报给管理员。
  后端补上启动器版本、系统、Java 版本，按 2 MB 上限裁剪（崩溃报告留开头，日志留结尾）。
- 测试：`cargo test --release -p theseus --lib galaxy` 32 项通过（新加 9 项：接口地址校验、路径白名单、内容裁剪、崩溃包大小、画质读写 / 备份恢复、
  同步规则）。插件在 Paper 1.21.1 测试服上冒烟 38 项通过。界面用 `_devtools/ui-harness/serve-api.mjs`（`make-serve-api.py` 生成，
  `?api=ok|down|maint|off&player=known|new|down&onboard=reset`）验证。
- 构建：debug 构建会跑 app-lib 的 Gradle，而 `galaxy-env.ps1` 的 JAVA_HOME 含中文路径，gradlew.bat 认不出来；构建命令前加
  `set JAVA_HOME=D:\GalaxyPokemon\_devtools\jdk21&&`（指向同一个 JDK 21 的目录联接）。
- 出包（2026-11-06 01:29，正式服关闭后由 `_devtools\slim\release-107.ps1` 自动跑）：release 构建 16 分钟，看门狗 0 次 kill。
  `release\安装程序\Galaxy宝可梦安装程序_v1.0.7.exe`（1980 MB，sha256 C09BF996…BB7BFC0）、`release\直装包\GalaxyPokemon_v1.0.7.zip`
  （2059 MB，sha256 AC6232AF…3BE814）；zip 里的 `launcher\GalaxyPokemon.exe` 和构建产物一致（763607B4…F90324），
  `galaxy-launcher.json` 带 `apiBase`。1.0.6 的两个包已删。没有经梦鱼推送。
- 插件实测：正式服 11-06 00:05 开服时加载，`127.0.0.1:8095` 返回真实数据（在线名单、TPS、赛季到 12-28、公告 7 条、Mai_Dang 的余额 / 点券 / 等级 / 通行证 / 签到）。
