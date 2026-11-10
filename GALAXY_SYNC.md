# Galaxy宝可梦: built-in modpack updater (replaces the DreamingFish in-game agent) — 2026-11-02

Owner request: "把梦鱼的逻辑和启动器合为一体，打开启动器时就检测更新，在启动器里检测和下载，UI 也用启动器的".
Today the DreamingFish `-javaagent` runs inside the game process at launch, opens its own window, and blocks the game
until it has synced (which also forced the 30-minute launch timeout hack). New design: the launcher itself is the
DreamingFish client. **The server side stays exactly as it is** (DreamingFishAdmin `serve` on :8080, public
`http://mc9.yunmaoc.com:28080`, releases published with `GalaxyPokemon-Launcher\scripts\dfs-release.ps1`).
Old players don't matter (no migration from the DreamingFish player's own state needed).

## Protocol facts (verified against the live server and real files — see the investigation summary)
- Trust anchor: `<instance>\.dreamingfish-bootstrap\project-binding.json` (UTF-8, Jackson pretty JSON):
  `{ "baseUrl":"http://mc9.yunmaoc.com:28080", "projectId":"galaxy-pokemon",
     "publicKey":"MCowBQYDK2VwAyEAPqMOIqOlK/TQgA+Bihv3Pt0bms91Kf77iMDOfKv+dTA=", "playerHome":"DreamingFishUpdater",
     "schemaVersion":1, "fallbackBranding":{…} }`.
  `publicKey` = base64 X.509 SubjectPublicKeyInfo DER (44 bytes, prefix `302a300506032b6570032100`); raw Ed25519 key =
  last 32 bytes. Also hard-code these three values as constants in the launcher and use them when the binding file is
  missing; if the file exists, its values win (lets the owner move the server). Reject a binding whose publicKey
  differs from the hard-coded one ONLY IF … no — keep it simple: binding file wins, constants are the fallback.
- `GET {baseUrl}/v1/projects/{projectId}/latest` → 200 `application/json`, body = the release manifest (raw bytes,
  ~320 KB). Headers: `ETag: "<sha256 of body>"`, `X-Dfs-Signature: <base64 64-byte Ed25519 signature>`,
  `Cache-Control: no-cache`. If the header is missing: `GET {uri}.sig` (base64 text + "\n", trim). Send
  `Accept: application/json` and `Accept-Encoding: identity`. Verify Ed25519 **strictly over the exact raw body bytes**
  (never re-serialize). Size limit 32 MiB.
- `GET {baseUrl}/v1/objects/sha256/{hex}` → 200 octet-stream, immutable, `Accept-Ranges: bytes`; `Range: bytes=N-`
  → 206 with `Content-Range`. Unknown → 404 `{"error":"object_not_found"}`. Server limit: 16 concurrent downloads
  total, overload → 503 + `Retry-After: 2`; retry 429/503 twice honouring Retry-After. Use ≤ 4 parallel downloads.
- `GET {baseUrl}/v1/projects/{projectId}/history` → unsigned `{projectId, releases:[{changelog,createdAt,
  displayVersion,releaseId,sequence}…newest first], schemaVersion}` (optional, for the UI changelog).
- `/healthz` → `{"status":"ok",…}`.
- Release manifest (schemaVersion 1), top-level keys: branding{…}, changelog, cleanupDirectories[], corrections[],
  createdAt, displayVersion ("1.0.5"), files[], forcedSyncDirectories[], forcedSyncFiles[], minimumPlayerVersion
  ("0.3.2"), optionalGroups[], projectId, releaseId ("r000007-20261031-174732-fef3f576"), releasedPaths[],
  requiredCapabilities[], retainedSelfManagedPaths[], schemaVersion, sequence (7), withdrawals[].
  - file entry: `{path, sha256, size, policy ("ENFORCED"|"LEGACY_MISSING_ONLY"), preset ("SYNC"|"REQUIRED"|"INITIAL"|
    "DEFAULT_CONFIG"), executable, componentId?, displayName?, optionalGroup?, version?}`. Paths are relative to the
    instance dir (`<.minecraft>\versions\1.21.1-Fabric 0.19.3`), '/'-separated, e.g. `mods/x.jar`,
    `config/…`, `options.txt` (INITIAL), `servers.dat` (INITIAL), `launcher-update/latest.json`,
    `launcher-update/launcher.zip`.
  - withdrawal: `{id, kind:"REMOVAL"|"VERSION", reason, createdAt, items:[{path, sha256, size, componentId?, version?}]}`.
  - correction: `{id, path, mode:"KNOWN_BAD"|"ONCE", badSha256[], reason, createdAt}`.
  - optionalGroup: `{id, title, description, defaultInstall}`.
  - Capabilities the DreamingFish client supports (accept exactly these): `forced-directory-sync-v1`,
    `forced-file-sync-v1`, `released-paths-v1`, `maintenance-policy-v2`, `simplified-maintenance-v1`.
- Bundled baseline in every install: `<instance>\.dreamingfish-bootstrap\bundled-release\manifest.json` + `manifest.sig`
  (byte-identical to the release the package was built from; .sig has a trailing "\n").

## Behaviour to implement (Rust, packages/app-lib, new module `api/galaxy_sync.rs`)
1. **State** (launcher-owned, never write DreamingFish's own state files): `<instance>\.galaxy-sync\`
   - `state.json`: `{schemaVersion:1, projectId, releaseId, sequence, manifestSha256, highestSequence,
     displayVersion, files:[{path, sha256, size}], initializedPaths:[…], syncedAt}` — the last fully applied release.
   - `release-manifest.json` + `.sig`: raw copy of the applied signed manifest.
   - `file-index.json`: hash cache `{path: {size, mtimeMs, sha256}}` so routine checks don't re-hash ~1200 files /
     ~700 MB; a file is re-hashed when size or mtime differs. "修复游戏文件" forces a full re-hash.
   - `downloads\` (`.part` files), `backups\<yyyyMMdd-HHmmss>_<releaseId>\` (old/removed files moved here, with a
     small `index.json` of {path, reason}). Keep only the newest 3 backup folders (prune older ones after a
     successful sync) so disk usage stays bounded.
   - Write state files atomically (temp + rename).
2. **Check** (`galaxy_sync_check`): read binding → fetch latest (10 s connect / 60 s total timeout, via the app's
   configured HTTP client so proxy settings apply) → verify signature → validate: schemaVersion 1, projectId matches,
   every requiredCapability is in the supported list (else status "needs newer launcher"), minimumPlayerVersion ≤
   "0.3.2" (semver compare; we declare player-compat 0.3.2), anti-replay: sequence ≥ state.highestSequence and an
   equal sequence must have the same manifest sha256 (else block: "服务器返回的版本比本地旧/被篡改"). If there is no
   state yet, the baseline is the bundled-release manifest (verify its signature too; if it is valid use it as the
   starting "highestSequence"). Then **plan** against the local files (using the hash cache) and return
   `{status: "upToDate"|"updateAvailable"|"offline"|"blocked", localVersion, remoteVersion (displayVersion),
   changelog, filesToDownload, bytesToDownload, filesToRemove, message?}`.
3. **Plan rules** (resolve every path inside the instance dir; reject absolute paths, `..`, drive letters, or
   anything escaping the instance → status blocked "PATH_UNSAFE"; compare paths case-insensitively on Windows):
   - Files whose `optionalGroup` belongs to a group with `defaultInstall:false` → not installed (treat as absent from
     the manifest). Others normal.
   - preset SYNC / REQUIRED (and policy LEGACY_MISSING_ONLY → treat as INITIAL): if local missing or sha differs →
     download + install (existing different file goes to backup first).
   - preset INITIAL / DEFAULT_CONFIG: if path ∈ state.initializedPaths → never touch. Else if the local file exists →
     keep it, add to initializedPaths. Else → download + install, add to initializedPaths. (On the very first run,
     options.txt / servers.dat already exist from the installer → they just get recorded.)
   - Removed since the previously applied release: a path in `state.files` that is not in the new manifest → if the
     local file still has the old sha256 → move to backup; if the player changed it → leave it.
   - withdrawals (REMOVAL and VERSION): for each item whose path is not a current manifest file, if the local file has
     sha256 == item.sha256 → move to backup.
   - corrections KNOWN_BAD: local file at `path` with sha in `badSha256` → backup (it is then reinstalled if it is a
     manifest file). ONCE: apply once (record the id in state.appliedCorrections).
   - forcedSyncDirectories / cleanupDirectories (currently empty, implement anyway): files under those dirs that are
     not manifest files → backup. forcedSyncFiles behave like SYNC. releasedPaths / retainedSelfManagedPaths → stop
     managing (never touch).
   - Never delete or touch anything else (worlds, screenshots, logs, player-added mods outside cleanup dirs, our
     `.galaxy-sync`, `.dreamingfish-bootstrap`).
   - DUPLICATE-modid detection is NOT required.
4. **Apply** (`galaxy_sync_apply`): refuse while a Galaxy game process is running ("请先关闭游戏") and hold an
   in-process lock so check/apply/launch can't overlap. Download objects to `downloads\<sha>.part` with resume
   (Range), verify size + sha256 (mismatch → delete part, retry once, then fail), ≤4 parallel, emit progress events
   (`galaxy_sync_progress`: {phase:"checking"|"hashing"|"downloading"|"applying"|"done"|"error", doneFiles,
   totalFiles, doneBytes, totalBytes, currentFile?, message?}) at most ~5×/s. After ALL downloads succeeded: move
   files that will be replaced/removed into the backup folder, then rename the verified downloads into place
   (same volume → atomic rename; create parent dirs). If any step of the apply phase fails, move backed-up files
   back (best effort) and report an error — never leave half a mods folder. Finally write state.json /
   release-manifest copy / file-index and prune old backups.
5. **Launch integration**: stop adding the DreamingFish `-javaagent` (packages/app-lib/src/launcher/mod.rs ~L2861; also
   strip an agent arg that a version JSON or custom args might carry, using `is_dreamingfish_agent_arg`). Remove the
   DreamingFish launch-timeout hack (`DREAMINGFISH_LAUNCH_TIMEOUT_SECS` / `launch_timeout_for` / its call in
   api/instance/run.rs and the slot in bootstrap) → stock timeouts again; keep the 1.0.4 change in state/process.rs
   (initialization wait = max(15 s, launch-preparation timeout)) — it is harmless. `GalaxyBootstrap.dreamingfish`
   becomes "the built-in updater is configured" (binding file or constants present → true). The
   galaxy_launch_blocked event/observer can stay (inert).
6. **Launcher self-update**: unchanged code path (`api/galaxy_update.rs` reads `<instance>\launcher-update\latest.json`,
   which the sync now writes). After a sync that changed `launcher-update/*`, the frontend re-runs the launcher update
   check so the "启动器有新版本" modal appears right away.
7. **Tauri commands** (apps/app/src/api/galaxy.rs + build.rs commands list; capability `galaxy:default` already covers
   the plugin): `galaxy_sync_check(force_rehash: bool)`, `galaxy_sync_apply()`, `galaxy_sync_status()` (last result,
   for page reloads). Event `galaxy_sync_progress`. Document them in GALAXY_CONTRACT.md.
8. Crypto/deps: prefer what app-lib already depends on directly (sha2, base64, serde_json, reqwest via the app's
   configured client, tokio). For Ed25519 use `ed25519-dalek 2.2` (already in Cargo.lock transitively) or `ring`
   (check which one is cheapest — must not require downloading new crates; run `cargo check` offline-style and look).

## Frontend (apps/app-frontend)
- `helpers/galaxy.ts` wrappers + `store/galaxy.ts` sync state. On app start (after `galaxy.bootstrap()` resolves with
  a game), run `galaxy_sync_check(false)`; if `updateAvailable` → **start `galaxy_sync_apply()` automatically**
  (the owner wants it to just happen when the launcher opens). Re-check when the window regains focus after > 30 min
  idle, and before launching (cheap: uses the hash cache).
- Home play dock (pages/Index.vue): replace "梦鱼自动更新已启用" with live sync state:
  - checking → "正在检查整合包更新…"; hashing (first run) → "首次检查整合包文件 n/N…";
  - downloading → progress bar + "正在更新整合包 12/40 · 35/120 MB"; applying → "正在安装更新…";
  - upToDate → "整合包已是最新 · 1.0.5" (click → small popover/modal with the release changelog from the manifest and
    the /history list if available);
  - offline → "暂时连不上更新服务器，将使用本地整合包" (play allowed);
  - blocked/error → red text with the reason + "重试" button (play allowed only if local files match the last applied
    state, otherwise disabled with "整合包不完整，请联网更新" — keep it simple: allow play on offline, disable on
    blocked/error).
  - The 开始游戏 button shows "更新中…" (disabled) while a sync runs; when the player clicks 开始游戏 the launcher first
    runs a quick check and applies any pending update, then launches.
- "修复游戏文件" quick action: also run `galaxy_sync_check(true)` + apply (full re-hash) after the existing repair.
- Remove/reword the DreamingFish-specific texts: home "梦鱼…" strings, the 1.0.4 "梦鱼正在检查整合包更新…" launching
  hint (no longer needed), MinecraftCrashModal timeout texts that mention DreamingFish downloading, the
  galaxy/announcements-bundled.ts entry about 梦鱼 if it describes the in-game updater window (reword to the launcher
  doing updates), App.vue "梦鱼更新器阻止了启动" notification can stay (inert).
- Keep the white/green layout v2 style; Chinese UI text.

## Packaging (GalaxyPokemon-Launcher, PowerShell 5 scripts with UTF-8 BOM)
- `scripts\dfs-release.ps1 -Deploy`: copy only `.dreamingfish-bootstrap\project-binding.json` and
  `.dreamingfish-bootstrap\bundled-release\*` into `client\...\versions\<ver>\.dreamingfish-bootstrap\`; no longer copy
  `bootstrap-agent.jar` nor `DreamingFishUpdater\`; remove those two from the client folder if present. Update the
  header comments ("玩家下次打开启动器时自动更新").
- Remove now-unneeded `client\.minecraft\versions\1.21.1-Fabric 0.19.3\DreamingFishUpdater\` (64 MB) and
  `.dreamingfish-bootstrap\bootstrap-agent.jar` from the client folder (back them up to
  `D:\GalaxyPokemon\_devtools\sync-backup\` first). Keep `project-binding.json` + `bundled-release\`.
- Do not publish a release and do not rebuild installers (I'll do that after verification).

## Verification (mandatory)
- `cargo check -p theseus_gui --release` and the frontend build pass. Use the memory-safe wrapper for every cargo/tauri
  build: `powershell -ExecutionPolicy Bypass -File D:\GalaxyPokemon\_devtools\slim\run-build.ps1 -Name <name> -Command
  "<cmd>" -Jobs 1 -TrimBelowMB 4200` (read it first; it waits for free RAM, trims rustc, kills only its own build if
  free RAM < 3 GB; output in `_devtools\slim\<name>.log`). Two live Minecraft servers run here — never touch java.exe.
- Release build + `galaxy-stage.ps1` (keep version 1.0.4 for now).
- End-to-end test WITHOUT launching the game: copy `D:\GalaxyPokemon\GalaxyPokemon-Launcher\client\.minecraft` to
  `D:\GalaxyPokemon\_devtools\sync-test\.minecraft` (≈2.9 GB, D: has ~11 GB free; delete it afterwards), run
  `target\release\GalaxyPokemon.exe` with `GALAXY_GAME_ROOT` pointing at it (and a separate app-data snapshot as
  previous tests did: move `%APPDATA%\com.galaxypokemon.launcher` aside and restore it afterwards), and verify via the
  launcher log + the files on disk + UI Automation (Win32 UIA can read button names / text):
  1. first open: hashing → up to date with r7 (displayVersion 1.0.5), 0 downloads, state.json written, options.txt /
     servers.dat recorded in initializedPaths;
  2. delete one mod jar and corrupt one config file → reopen → update detected (2 files) → auto-downloaded from the
     public server, sha verified, files restored, originals of the corrupted file in backups\;
  3. edit options.txt → reopen → NOT overwritten;
  4. add an extra jar to mods\ → reopen → left alone;
  5. offline: set `GALAXY_GAME_ROOT` copy's binding baseUrl to an unreachable port → "offline" status, play allowed;
  6. tamper: change one byte of the cached release-manifest copy / or point to a fake latest → signature failure → blocked.
  (If UIA can't read the dock text, verify through the launcher log lines you add: log each sync phase with counts.)
- Harness screenshots (D:\GalaxyPokemon\_devtools\ui-harness, serve.mjs mocks — add mocks for the new commands/events
  with opt-in URL flags like `?sync=downloading`) of the home dock in each state: v7-sync-*.png.
- UTF-8 no BOM + LF for repo files (BOM for the PowerShell scripts), no mojibake; back up every file before editing to
  `D:\GalaxyPokemon\_devtools\sync-backup\` (no git here).
