# Backend progress log (Galaxy宝可梦)

## Session 2 (resumed after interruption)
- Reviewed: app-lib api/galaxy.rs (bootstrap, set_player, DreamingFish agent arg, blocked-line detection + tests),
  apps/app api/galaxy.rs plugin (registered in main.rs, `galaxy:default` in capabilities/plugins.json),
  launcher/mod.rs agent hook (absolute path, de-dup, cwd = instance dir), process.rs output hook, emit.rs event.
- Goal 1 (rebrand): brand.rs + tests done earlier; brand-guard.mjs required values switched to the fixed values
  (also checks mainBinaryName, main window title, no updater plugin block). Forbidden patterns kept.
- Goal 2 (privacy): settings defaults off, telemetry ENDPOINT = None, updater feature not default and no updater
  plugin block. Migration 20261104120000 registered in downgrade-app-db.mjs REVERTIBLE_SCHEMA as `{}`.
- Goal 5: cargo check -p theseus_gui running (log: D:\GalaxyPokemon\_devtools\cargo-check.log).
- Goal 5: fixed galaxy.rs java_path patch type (`Some(java_path)`). Build needs MinGW gcc for the seed-map DLL:
  set `$env:AXOLOTL_CUBIOMES_CC = 'D:\GalaxyPokemon\_devtools\w64devkit\bin\gcc.exe'` after dot-sourcing galaxy-env.ps1.
  `cargo check -p theseus_gui` → Finished, exit 0. Remaining warnings are pre-existing (none in galaxy/launcher/process code).
- Brand guard: reworded "Modrinth App" attribution in Cargo.toml / installer.nsi / tauri.conf.json copyright
  (forbidden pattern). Only failure left: config.ts UA (frontend, see GALAXY_TODO.md).
- downgrade-app-db.test.mjs passes. Next: cargo test -p theseus --lib -- brand:: api::galaxy:: (log: _devtools\cargo-test.log).
- Tests: `cargo test -p theseus --lib -- brand:: api::galaxy::` → 15 passed. Needs
  `$env:GRADLE_USER_HOME = 'D:\GalaxyPokemon\_devtools\gradle-home'` (Aliyun mirror init script lives there; without it
  Gradle writes ~320 MB to C:\Users\Administrator\.gradle and fails on Maven Central 403; that stray dir was removed).
- All 5 goals done. Open: config.ts UA (frontend TODO), rustfmt not installed (format not verified),
  updater_impl.rs still has Axolotl URLs but is only compiled with `--features updater` (never pass it /
  tauri-updater.conf.json when packaging).

## Session 3: "Tried to run a nonexistent instance" on Play
- Root cause: the id from bootstrap IS the SQL `instances.id` that run/get use (same pool, `local:<uuid>`). The row was
  deleted right after creation by direct-link reconciliation: App.vue `startConfiguredDirectLinkSync` calls
  `instance_sync_direct_links(roots from Settings)` on start and on every window focus; `sync_direct_link_instances`
  removes every direct-link record whose `linked_dot_minecraft` is not a configured root. Our root is never configured
  there, so the instance vanished (app.db had 0 instances after the failed run) and a new one was created on each start.
- Fix: `galaxy::is_bootstrap_link(&InstanceMetadata)` (api/galaxy.rs) + skip such records in the removal loop of
  state/instances/commands/sync_direct_link_instances.rs. No frontend change.
- "User is not logged in" at startup = NoCredentialsError from skin APIs before any account exists; harmless.
- run with offline_mode=false uses Credentials::get_default_credential = active account (set by galaxy_set_player);
  refresh() is a no-op for offline accounts; anti-piracy gate skipped via exempt_instance.
- Next: release build (log _devtools\tauri-build2.log), clean run + reuse run with ui-play.ps1.
- Step 1 done: release build OK (EXIT=0, exe 2026-10-31T19:30). Next: clean run (delete %APPDATA%\com.galaxypokemon.launcher) + ui-play.ps1.
- (correction: exe built 2026-10-31T19:18:50)
- Step 2/3/4 clean run PASSED (19:20): instance local:e9b90f4a-... survived frontend sync; javaw from
  java-runtime-delta, DreamingFish -javaagent + --quickPlayMultiplayer mc9.yunmaoc.com:27852, --uuid matches
  nameUUIDFromBytes("OfflinePlayer:Galaxy_Test"); DreamingFish "已允许 Minecraft 启动"; latest.log "Loading Minecraft 1.21.1
  with Fabric Loader 0.19.5". javaw stopped (only the client one). Next: reuse run (restart launcher, no data delete).
- Step 2/3/4 reuse run PASSED (19:22): no "Linking" line, still exactly one instance (same id); no name dialog;
  javaw again had agent + quick play; DreamingFish permit 19:23:03; Fabric loading 19:23:04. javaw 23336 stopped,
  server java.exe 2484/4308 untouched. Launcher stopped.
- Step 6 cleanup done (logs, crash-reports, DreamingFishUpdater\{logs,cache,staging,backups}, game.lock removed).
  NOT removed (created by the test game run, outside the requested list): .mixin.out, cobblemon-battle-logs,
  defaultconfigs, downloads, gyms, moddata, xaero; options.txt/config touched.
- Open: version JSON's `--quickPlayPath ${quickPlayPath}` is passed unsubstituted (pre-existing, harmless so far);
  theseus.jar agent is extracted to system %TEMP% (C:) at launch.
