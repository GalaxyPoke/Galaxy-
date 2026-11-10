# 两个工程师之间的约定（后端 / 前端分工）

需求见 GALAXY_FORK.md。两个人**同时**改代码，各自只改自己负责的文件，接口按本文件来。

## 固定值

| 项 | 值 |
|---|---|
| 产品名 productName / 窗口标题 | `Galaxy宝可梦` |
| 英文短名（UA、文件名等需要 ASCII 处） | `GalaxyPokemon` |
| identifier（决定数据目录 `%APPDATA%\com.galaxypokemon.launcher`） | `com.galaxypokemon.launcher` |
| deep-link scheme | `galaxypokemon` |
| User-Agent | `GalaxyPokemon-Launcher/${version} (${os})` |
| 服务器地址 | `mc9.yunmaoc.com:27852` |
| 实例显示名 | `Galaxy宝可梦` |
| 版本 id（版本 json 名） | `1.21.1-Fabric 0.19.3` |
| 图标源文件 | `D:\GalaxyPokemon\branding\icon.svg`、`icon-1024.png` |

## 安装布局（玩家电脑）

```
<安装目录>\
  Galaxy宝可梦.exe          ← 外层小入口，打开 launcher\ 里的程序（已有，不用管）
  launcher\                 ← 本启动器（tauri 打包产物放这里）
  .minecraft\               ← 整合包；versions\1.21.1-Fabric 0.19.3\ 是版本隔离实例目录
     runtime\java-runtime-delta\bin\javaw.exe   ← 自带 Java 21
     versions\1.21.1-Fabric 0.19.3\.dreamingfish-bootstrap\project-binding.json + bundled-release\
                                                 ← 梦鱼服务器的项目绑定和签名基线（启动器自带更新器用；不再有 bootstrap-agent.jar）
     versions\1.21.1-Fabric 0.19.3\.galaxy-sync\ ← 启动器更新器自己的记录（state.json、哈希缓存、下载、备份）
```

找 `.minecraft` 的顺序：环境变量 `GALAXY_GAME_ROOT` → exe 所在目录的 `..\.minecraft` → exe 所在目录的 `.minecraft`。
开发时设 `GALAXY_GAME_ROOT=D:\GalaxyPokemon\GalaxyPokemon-Launcher\client\.minecraft`。

## 后端提供给前端的 Tauri 命令（新插件 `galaxy`，后端负责实现与注册）

前端用 `invoke('plugin:galaxy|<name>', args)` 调用，前端在 `apps/app-frontend/src/helpers/galaxy.ts` 封装。

- `galaxy_bootstrap() -> GalaxyBootstrap`
  - 幂等：找到游戏目录；若还没有对应的关联实例，就用现有的「关联外部 .minecraft / 版本隔离」机制建一个（名字 `Galaxy宝可梦`，
    java_path 覆盖为自带 Java 21，若存在）；已有就复用（按 linked 路径匹配，不要重复建）。
  - 返回：
    ```ts
    {
      instanceId: string | null,        // 现有前端 run()/get() 用的 id（profile path / id，与 helpers/instance.ts 一致）
      serverAddress: string,            // "mc9.yunmaoc.com:27852"
      gameRoot: string | null,          // 找到的 .minecraft 绝对路径
      instanceDir: string | null,       // versions\1.21.1-Fabric 0.19.3 绝对路径
      gameFound: boolean,
      dreamingfish: boolean,            // 自带的整合包更新器已配置（找到游戏即为 true；2026-11-02 前表示 bootstrap-agent.jar 是否存在）
      javaPath: string | null,
      error: string | null              // 找不到游戏目录等，中文
    }
    ```
- `galaxy_set_player(name: string) -> { uuid: string }`
  - 校验 `^[A-Za-z0-9_]{3,16}$`（不合格返回中文错误）；创建或复用该名字的离线账户并设为当前账户
    （UUID = nameUUIDFromBytes("OfflinePlayer:"+name)，现有 Credentials::offline 已是此算法）。
    不受 anti-piracy「必须先有正版账户」的限制。

## 后端额外负责
- 品牌：tauri.conf*.json（productName、identifier、mainBinaryName 用 ASCII `GalaxyPokemon`、窗口标题、scheme、图标 → 用
  `pnpm tauri icon` 或手动从 icon-1024.png 生成 apps/app/icons/*）、brand.rs、Cargo 包描述、NSIS 安装器文字。
- 关闭 Tauri updater（去掉 updater feature/插件块或让 endpoints 为空且不报错），关闭 Discord RPC、遥测、playtime/analytics 上报默认值。
- ~~启动时追加梦鱼 `-javaagent`~~（2026-11-02 起不再加载，版本 json / 自定义参数里带的也会去掉；整合包由启动器自带更新器
  在启动前同步，见文末「Built-in modpack updater」），Java 进程 cwd = 实例目录。
- 进程输出里出现 `DreamingFish updater blocked Minecraft startup` 时，发一个事件给前端：
  `emit('galaxy_launch_blocked', { instanceId, reason })`（reason 取该行冒号后内容）。现在不加载 agent，这个事件不会再出现，保留无害。
- `scripts/axolotl/brand-guard.mjs` 的「必须是 Axolotl」类断言改成上面的固定值，「禁止出现」类检查保留。
  **config.ts 由前端改**，后端只改 guard 脚本里对 config.ts 的期望值（UA 字面量、`privateModrinthServices: false`、`ghsTelemetry: false` 保持）。

## 前端负责（apps/app-frontend、packages/ui 的文案）
- 路由/导航：只留 首页、游戏（固定实例页）、皮肤（可选）、设置；去掉 Discover/Browse/项目页、Library、Create、Multiplayer、Lab、
  Worlds、Downloads、Help/Drop、headless-demo、QuickInstanceSwitcher、创建实例按钮、Modrinth 账户菜单。
- 启动时调用 `galaxy_bootstrap`，把 instanceId 存进一个 store；「游戏」导航直达 `/instance/<id>`。
- 首页重写：服务器卡片（状态用现有 `fetchCachedServerStatus`/`get_server_status`，Play = 现有 `run(id, serverAddress)`）、公告（本地静态列表即可，后面再接远程）。
- 账户：只保留离线账户，输入框 + 校验，调用 `galaxy_set_player`；去掉 Microsoft / Yggdrasil 入口。
- 实例页：保留 内容（只读，去掉添加/更新/删除/浏览）、截图、日志；设置弹窗隐藏 Installation/Sync/Backup/删除实例。
- 去掉远程公告（admin.axlmc.org）、Axolotl 更新公告弹窗、更新检查、隐私同意弹窗里的 axlmc 链接（遥测默认关）、Minecraft 新闻、每日挑战等。
- 文案：config.ts 品牌（productName 等按上表），zh-CN / en-US 里用户可见的 "Axolotl"/"美西螈" 改为 Galaxy宝可梦
  （关于页保留「基于 Axolotl Launcher 与 Modrinth App，GPL-3.0」的署名）。监听 `galaxy_launch_blocked` 显示中文错误。

## 共同规则
- 只改自己那一侧的文件；需要对方改的写进 `GALAXY_TODO.md`（追加，不要覆盖）。
- 构建 / 命令前先 `. .\galaxy-env.ps1`（工具链都在 D 盘；**C 盘只剩约 3 GB，任何缓存 / 临时文件都不能写 C 盘**）。
- 不碰 `D:\GalaxyPokemon\GalaxyPokemon正式服`（线上服务器）和 `D:\GalaxyPokemon\GalaxyPokemon-Launcher`（旧启动器 + 出包脚本，只读参考）。
- 每次改完至少过一遍：后端 `cargo check -p theseus_gui`（首次很慢，正常）；前端 `pnpm --filter @modrinth/app-frontend exec vue-tsc --noEmit` 或项目已有的 lint/typecheck 脚本。

## Launcher self-update (added 2026-10-31)

Backend: `packages/app-lib/src/api/galaxy_update.rs`. Frontend: `helpers/galaxy.ts`, prompt modal in `App.vue`.

- `plugin:galaxy|galaxy_launcher_update_check` → `{ currentVersion: string|null, available, version, notes, size }`.
  `currentVersion` comes from `galaxy-launcher.json` next to the exe; `null` means self-update is off (dev builds).
- `plugin:galaxy|galaxy_launcher_update_apply` → new version string. Verifies SHA-256, extracts to
  `<launcher>\.galaxy-update`, spawns a hidden PowerShell that waits for this PID, robocopies over `launcher\`
  (log: `launcher\galaxy-update.log`) and starts `GalaxyPokemon.exe`; the app exits ~600 ms after returning.
- Channel: `<.minecraft>\versions\1.21.1-Fabric 0.19.3\launcher-update\latest.json`
  `{ version, notes, file: "launcher.zip", sha256, size }`, part of the signed DreamingFish release and synced by the
  launcher's built-in updater (`galaxy_sync`); after a sync that changed `launcher-update/*` the frontend re-runs the check.
- Release: `galaxy-stage.ps1` (writes `dist-launcher\galaxy-launcher.json`, version from
  `GalaxyPokemon-Launcher\package.json`), then `GalaxyPokemon-Launcher\scripts\dfs-release.ps1` zips `dist-launcher`.
- The frontend refuses to apply while a game process is running.

## Announcements source (added 2026-11-01)

Frontend: `apps/app-frontend/src/galaxy/announcement-source.ts`. Backend: `fetch_announcements` in
`packages/app-lib/src/api/galaxy.rs`, command `plugin:galaxy|galaxy_fetch_announcements(url) -> string` (body text).

- **Where to set it:** `export const GALAXY_ANNOUNCEMENTS_URL = ''` in announcement-source.ts. It is a build-time
  constant, not a user setting. Empty → the bundled list (`src/galaxy/announcements-bundled.ts`), no network.
- **Transport:** the backend GETs the URL with the app's configured HTTP client (`Accept: application/json`):
  http/https only, no credentials in the URL, 10 s timeout, max 1 MB, UTF-8. The webview CSP is not changed.
- **Response:** a JSON array (or `{ "announcements": [...] }`) of:
  ```jsonc
  {
    "id": "3",                       // string or number, unique
    "title": "&6万圣节捕捉狂欢开启",   // Minecraft &x / §x codes allowed
    "content": ["&f第一行", "&7第二行"], // string[] (a single string is split on newlines)
    "category": "活动预告",           // free text, default "公告"
    "pinned": false,
    "mustRead": false,
    "priority": 80,                  // higher first, after pinned
    "createdAt": "2026-10-30T18:00", // required, local time YYYY-MM-DD[THH:mm[:ss]]
    "startAt": "2026-10-30T18:00",   // optional, hidden before
    "endAt": "2026-11-02T23:59",     // optional, hidden after
    "enabled": true,                 // optional, false → dropped
    "button": { "text": "&6[查看活动]", "action": "RUN_COMMAND", "value": "/activity" } // optional
  }
  ```
  The plugin's kebab-case keys (`must-read`, `created-at`, `start-at`, `end-at`) are accepted too. Entries without
  `id`, `title` or a valid `createdAt` are dropped; at most 200 entries, 60 lines each. Buttons are display-only: for
  RUN_COMMAND / SUGGEST_COMMAND the launcher shows "进服后输入 /value".
- **Fallback:** a successful response is cached in localStorage (`galaxy.announcements.cache.v1`, tied to the URL);
  on failure the cache is shown ("暂时无法连接服务器，显示的是上次获取的公告"), otherwise the bundled list.

## Built-in modpack updater (added 2026-11-02)

Backend: `packages/app-lib/src/api/galaxy_sync.rs` (+ `galaxy_sync/engine.rs`). Frontend: `helpers/galaxy.ts`,
`store/galaxy.ts` (`sync()`), home play dock in `pages/Index.vue`. Spec: `GALAXY_SYNC.md`; details in GALAXY_FORK.md,
"Built-in updater (galaxy_sync)". All three commands are in the `galaxy` plugin (`galaxy:default` allows them).

- `plugin:galaxy|galaxy_sync_check({ forceRehash?: boolean }) -> GalaxySyncResult`
  Fetches `GET {baseUrl}/v1/projects/{projectId}/latest`, verifies the Ed25519 signature over the raw bytes, validates
  it (schema 1, project id, capabilities, `minimumPlayerVersion` ≤ 0.3.2, anti-replay) and plans against the local files
  through the hash cache (`forceRehash: true` re-hashes everything, used by 修复游戏文件). When nothing has to change the
  release is recorded right away (state.json). Never downloads. Rejects with "正在检查或更新整合包，请稍候" while another
  check / apply / launch holds the sync lock.
- `plugin:galaxy|galaxy_sync_apply() -> GalaxySyncResult`
  Same check (reuses the manifest verified in the last 5 minutes), then downloads, verifies and installs. Rejects with
  "请先关闭游戏，再更新整合包" while a game process runs, and with a Chinese message when a download or the install
  failed (everything already moved is put back first). Offline / blocked come back as a normal result.
- `plugin:galaxy|galaxy_sync_status() -> { busy: boolean, lastResult: GalaxySyncResult | null, progress: GalaxySyncProgress | null }`
  For page reloads (the store follows a running sync instead of starting another).
- Event `galaxy_sync_progress` (≤ ~5 per second, phase changes always):
  `{ phase: "checking"|"hashing"|"downloading"|"applying"|"done"|"error", doneFiles, totalFiles, doneBytes, totalBytes,
  currentFile: string|null, message: string|null }`. `hashing` only appears when files miss the hash cache.

```ts
GalaxySyncResult = {
  status: 'upToDate' | 'updateAvailable' | 'offline' | 'blocked' | 'error',
  code: string | null,      // OFFLINE, SIGNATURE_INVALID, SIGNATURE_MISSING, MANIFEST_INVALID, MANIFEST_TOO_LARGE, PATH_UNSAFE,
                            // PROJECT_MISMATCH, NEEDS_NEWER_LAUNCHER, REPLAY, LOCAL_STATE_INVALID, GAME_NOT_FOUND,
                            // BINDING_INVALID, CHECK_FAILED, STATE_WRITE_FAILED, APPLY_FAILED (frontend: INVOKE_FAILED)
  message: string | null,   // Chinese, for the UI
  localVersion, remoteVersion,            // displayVersion of the applied release / of the server's release
  releaseId, sequence, changelog, releasedAt,
  filesToDownload, bytesToDownload, filesToRemove, hashedFiles,
  applied, filesInstalled, filesBackedUp, backupDir,   // set by apply
  launcherUpdateChanged: boolean,                      // launcher-update/* changed → re-run galaxy_launcher_update_check
  history: { releaseId, sequence, displayVersion, changelog, createdAt }[],  // GET /history, best effort, newest first
  checkedAt: string,
}
```

- Play rule (frontend): `offline` → play allowed (local pack); `blocked` / `error` → 开始游戏 disabled until a retry
  succeeds; while a sync runs the button reads 更新中…. Clicking 开始游戏 runs `sync()` (check + apply) first. The backend
  launch path (`api/instance/run.rs`) takes the same lock and refuses to start while a sync runs.

## Server data API, graphics presets (added 2026-11-05, launcher 1.0.7)

Server side: the GalaxyLauncherApi plugin on the main server; endpoints and fields in `GALAXY_LAUNCHER_API.md`.
Backend: `packages/app-lib/src/api/galaxy_api.rs`, `galaxy_graphics.rs`. Frontend: `helpers/galaxy.ts`,
`src/galaxy/server-api.ts`, `use-server-data.ts`, home page `pages/Index.vue`, crash dialog `MinecraftCrashModal.vue`.

- `GalaxyBootstrap.apiBase: string | null`: `GALAXY_API_BASE`, else `apiBase` in `launcher\galaxy-launcher.json`
  (written by `galaxy-stage.ps1 -ApiBase`, default `http://mc9.yunmaoc.com:28095`). http/https, no credentials,
  query or fragment; trailing `/` removed. Null → the features below are hidden.
- `plugin:galaxy|galaxy_api_get({ path }) -> string`: GET `<apiBase><path>` for exactly `/v1/health`, `/v1/summary`,
  `/v1/announcements` or `/v1/player/<3–16 of [A-Za-z0-9_]>`; 8 s timeout, 1 MB cap; non-2xx → the response's
  `error` text.
- Announcements: with `GALAXY_ANNOUNCEMENTS_URL` empty the frontend now uses `<apiBase>/v1/announcements`
  through `galaxy_fetch_announcements` (same cache and fallback rules as above).
- `plugin:galaxy|galaxy_crash_upload({ report: { name, summary, modpackVersion, memoryMb, files: [{ name, content }] } })
  -> { id }`: the backend adds `launcherVersion`, `os`, `javaVersion` (bundled runtime's `release` file), keeps at most
  4 files of 1 MB each (crash reports keep the start, logs the end), shrinks the largest file until the JSON is under
  ~1.94 MB, and POSTs `<apiBase>/v1/crash` (45 s timeout).
- `plugin:galaxy|galaxy_graphics_get() -> GalaxyGraphicsState`,
  `plugin:galaxy|galaxy_graphics_apply({ preset: 'low' | 'balanced' | 'high' | 'custom' }) -> GalaxyGraphicsState`:
  ```ts
  { preset: 'low' | 'balanced' | 'high' | null, optionsFound, renderDistance: number | null,
    shadersEnabled, shaderPack: string | null, shaderPackAvailable, hasBackup }
  ```
  Rewrites the video keys of `<instance>\options.txt` and `enableShaders` / `shaderPack` in `config\iris.properties`
  (other lines kept, atomic write). Own settings (matching no preset) are copied to `<instance>\.galaxy-graphics\`
  first; `custom` copies them back. Refused while any game process runs.
- Sync rule: `config/iris.properties` and `config/sodium-options.json` are treated as INITIAL even when the release
  ships them as SYNC (`PLAYER_PREFERENCE_FILES` in `galaxy_sync/engine.rs`); `forcedSyncFiles` and corrections still
  apply.
