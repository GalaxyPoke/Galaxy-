# Galaxy宝可梦 launcher slimming (2026-11-01)

Owner request: "瘦身吧". The launcher exe is 184.96 MB (`dist-launcher\GalaxyPokemon.exe`, release build,
opt-level "s", strip, NO LTO), the self-update zip is ~76 MB, the frontend `apps/app-frontend/dist` is 50.4 MB / 884 files
(embedded into the exe by Tauri). Goal: make it much smaller WITHOUT breaking anything the Galaxy build actually uses.

## Baseline facts (measured)
- Largest dist files: `dist/monaco/**` (Monaco editor + TS/CSS/HTML/JSON workers, ~20 MB total, e.g. ts.worker 6.7 MB +
  6.4 MB duplicate), `dist/easteregg/games/game.html` 6.3 MB, `assets/agent-*.js` 1.3 MB (AI agent), vendor-three 0.7 MB
  (needed by the skin preview — keep), zh-TW/other locale chunks.
- Largest Rust rlibs: theseus (app-lib) 389 MB, windows 121 MB, rmcp (MCP server) 23 MB, zhconv (zh-TW conversion
  tables) 11.6 MB, launcher_tools 9.6 MB, zbus, moxcms …
- `apps/app/src/seed_map/**` embeds ~2.5 MB via include_bytes; `dist-launcher/resources/seed-map/axolotl-cubiomes.dll`
  2.5 MB; `resources/blockbench-skin/README.md`.
- Build machine: 24 GB RAM, only ~8 GB free (two live Minecraft servers run here — NEVER let a build starve them),
  6 threads, C: has ~5.8 GB free (pagefile lives there).

## What the Galaxy build must keep working (do not remove)
Home (server lobby: galaxy bootstrap, `plugin:worlds|get_server_status`, process listener, run/kill, name dialog),
announcements page + `galaxy_fetch_announcements`, launcher self-update (`galaxy_launcher_update_*`), offline accounts
(auth plugin offline paths, AccountsCard/PlayerChip), instance page tabs 内容/截图/日志 (read-only content list,
screenshots, logs), instance settings modal (General/Window/Java/Launch-prep: memory, java path, resolution, args),
repair (`install_existing_instance`), open folder / copy path, Java handling for the bundled runtime, download manager /
install jobs used by repair & launch, Skins page (offline skins via resource pack, skin preview with three.js, add/upload
skin), Settings categories still visible (interface, shortcut-settings, language-translation, launch-defaults,
java-performance, storage-backups, privacy-data, about, network-connection, logs), notifications, tray/close behaviour,
window controls, deep links `galaxypokemon://` (privacy settings mention them — keep the handler), anti_piracy exemption
used by galaxy bootstrap, direct-link instance sync (`sync_direct_link_instances` + is_bootstrap_link), crash modal,
error modal, onboarding overlay (Galaxy steps), lightweight/tray mode, i18n zh-CN/en-US (zh-TW may go — see below).

## Candidates to remove (verify each is unreachable in the Galaxy UI first)
Frontend:
- Monaco editor + workers (`dist/monaco`): find where it comes from (vite plugin / public dir / static copy) and what
  imports it (file studio, schematic/lab, config editor…). Remove if only used by unreachable features.
- `easteregg/` games, AboutAxolotlRun / axolotl-run / axolotl-balls assets, `announcements/catalog.ts` (5,900-line
  Axolotl changelog) + UpdateAnnouncement* components, remote announcements, app-update-button (gated off).
- Modals/flows still mounted in App.vue but unreachable: CreationFlowModal, ContentInstallModal,
  ModpackAlreadyInstalledModal, AddServerToInstanceModal, ContentUpdaterModal, ModpackInstallModal,
  CurseForgeManualDownloadsModal, drop/import overlays + ConfirmDropTypeModal, GenericContentInstallModal,
  InstanceExportModal, LauncherImportModal, SymlinkMethodCards, BatchScanOverlay, McpConnectionOverlay /
  McpConnectionStatus / startMcpStateSync / startMcpUiBridge, I18nDebugPanel (dev), QuickInstanceSwitcher,
  NavRail/NavButton (only specs use them).
- Unregistered route pages and their exclusive components: Browse, project/*, library, worlds/WorldEditor, multiplayer,
  lab/*, downloads page, FileStudio, instance Files/Overview/upgrade, SkinEditor, headless-demo, discover.
- AI settings/agent (`assets/agent-*`), MCP settings, instance-sync, content-downloads settings (already hidden).
- zh-TW locale: keep only if the language setting offers it; if removing, also remove the zhconv crate usage (see Rust).
Rust (apps/app + packages/app-lib):
- Tauri plugins/commands with no Galaxy UI: mcp (rmcp), seed_map (+ cubiomes build step, include_bytes assets, the
  resources/seed-map dll, AXOLOTL_CUBIOMES_CC in galaxy-env.ps1 can stay harmless), AI/agent, servers hosting/lifecycle,
  terracotta / hongshi (P2P/online-play tunnels — check what they are), curseforge, modrinth browse/project/search,
  lab tools (schematic preview, recipe generator), file studio, instance export / launcher import, world editing
  (KEEP `get_server_status` and whatever the home page uses from the worlds plugin), multiplayer server-list sync,
  blockbench skin editor resources, analytics, app updater (already feature-gated off).
- Unregistering a plugin = remove `.plugin(api::x::init())` in main.rs, its `mod` in api/mod.rs if unused, its entry in
  build.rs `InlinedPlugin` list, and `"x:default"` in `apps/app/capabilities/*.json`. Also stop background tasks started at
  state init / setup for removed features (they keep code alive). The linker (/OPT:REF) drops app-lib code that becomes
  unreferenced — deleting app-lib source is NOT required; prefer the smallest diff that unregisters/stops things.
  Remove now-unused Cargo deps from apps/app/Cargo.toml only when cargo says they are unused (don't touch app-lib's).
- Keep upstream modules compiling; don't do sweeping refactors.

## Build profile
After feature removal, measure, then try `lto = "thin"` (and optionally `codegen-units = 1`) via environment variables
for the release build only (CARGO_PROFILE_RELEASE_LTO / CARGO_PROFILE_RELEASE_CODEGEN_UNITS) or in tauri-release flow —
document what you chose in galaxy-env.ps1 / GALAXY_FORK.md. Fat LTO only if a memory watchdog shows it fits.
MEMORY SAFETY (mandatory for every cargo/tauri build): run the build with `CARGO_BUILD_JOBS=3` and a watchdog that
checks free physical memory every 5 s and kills cargo/rustc/link.exe/lld processes it started if free RAM < 3 GB; then
fall back to a lighter setting. Never kill java.exe or anything you didn't start.

## Verify
- `pnpm --filter @modrinth/app-frontend run build` passes; `cargo check -p theseus_gui --release` passes;
  `node scripts/axolotl/brand-guard.mjs` (or however brand-guard runs) passes.
- Release build: `cmd /c "pnpm --filter @modrinth/app exec tauri build --config tauri-release.conf.json --no-bundle"`, then
  `powershell -ExecutionPolicy Bypass -File galaxy-stage.ps1`. Record exe size, dist size, file count, and zip size of
  dist-launcher (Compress-Archive to %TEMP% or the same method dfs-release.ps1 uses) before/after each phase.
- Smoke test the built exe for ~25 s (window alive, title Galaxy宝可梦, launcher log has the galaxy bootstrap line and no
  new ERROR/panic), then stop only that process and remove %APPDATA%\com.galaxypokemon.launcher and
  %LOCALAPPDATA%\com.galaxypokemon.launcher test data afterwards (the harness needs release\app.db — keep a copy for
  serve.mjs if you need it, or re-create it the same way).
- Harness screenshots (D:\GalaxyPokemon\_devtools\ui-harness: serve.mjs 4719 + headless Edge CDP 9222 + shoot-v4.mjs /
  shoot-v3.ps1) of /, /announcements, /instance/galaxy, /instance/galaxy/logs, /skins, /settings (+ about), player menu,
  name dialog — compare with the existing v4-*/v3-* shots: no regressions, no new console exceptions.
- Do NOT bump the version, do NOT publish via DreamingFish, do NOT rebuild installers (the owner decides later).
- UTF-8 no BOM, LF; scan touched files for mojibake.
