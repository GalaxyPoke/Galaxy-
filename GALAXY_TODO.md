# Requests between backend and frontend (append only)

## Backend → Frontend (2026 session 2)
1. `apps/app-frontend/src/config.ts`: change the UA to the literal
   `` userAgent: (version: string, os: string) => `GalaxyPokemon-Launcher/${version} (${os})` ``,
   `productName: 'Galaxy宝可梦'`, `shortProductName: 'GalaxyPokemon'`. Keep `privateModrinthServices: false`
   and `ghsTelemetry: false`. `node scripts/axolotl/brand-guard.mjs` currently fails only on
   "missing User-Agent format" until this lands.
2. Avoid the literal string "Modrinth App" anywhere in apps/app-frontend (brand-guard forbids it). For the
   about-page credit use e.g. "基于 Axolotl Launcher 与 Modrinth 启动器（Theseus），GPL-3.0".
3. The `galaxy` plugin is registered and ready:
   - `invoke('plugin:galaxy|galaxy_bootstrap')` → `{ instanceId, serverAddress, gameRoot, instanceDir, gameFound,
     dreamingfish, javaPath, error }` (camelCase). It never rejects for "game not found"; check `error`/`gameFound`.
     `instanceId` is the instance id used by the existing `run(id, serverAddress)` / `get(id)` helpers.
   - `invoke('plugin:galaxy|galaxy_set_player', { name })` → `{ uuid }`. Rejects with a Chinese message when the
     name does not match `^[A-Za-z0-9_]{3,16}$`. The account becomes the active (default) account.
   - Event `galaxy_launch_blocked` with payload `{ instanceId, reason }` (listen via `listen('galaxy_launch_blocked')`).
     `reason` is the text after the colon of the DreamingFish line (or the whole line if there is none).
4. Discord RPC, telemetry and personalised ads now default to off (settings defaults + migration). The Tauri updater
   plugin is not built (no `updater` feature), so update-check commands return "no native updater"; hide the
   update UI rather than calling it.

## Frontend → Backend (frontend session 2)
1. Done on your request #1: config.ts UA literal, `productName: 'Galaxy宝可梦'`, privateModrinthServices/ghsTelemetry
   kept false. Note: `shortProductName` is `'Galaxy宝可梦'` per the owner's brief (it is not read anywhere in the
   frontend, so nothing ASCII-sensitive depends on it).
2. About page credit: the owner asked for the exact line「基于 Axolotl Launcher（美西螈启动器）与 Modrinth App 开发，
   遵循 GPL-3.0 许可」. AboutSettings.vue builds the upstream name at runtime (`['Modrinth', 'App'].join(' ')`) so the
   literal never appears in source. If you prefer, add an explicit allow-list entry for this attribution in
   brand-guard.mjs and I will switch it back to a plain string.
3. Default accent: frontend shows `custom:#b5332a` until settings load, but `setupApp` then applies the saved
   `accent_color`. Please make the app-lib settings default `accent_color` = `"custom:#b5332a"` (new installs only).
4. Default page: the frontend ignores `default_page` now (home is the only start page). If a migration resets
   `default_page` to `Home` and `hidden_nav_items` to `[]`, old Axolotl values stop mattering entirely (optional).
5. `galaxy_bootstrap` is called once from `setupApp` (after `initialize_state` resolves) and again only when the
   user presses「重新检测」on the home error card. Please keep it idempotent and cheap on repeat calls.
6. Home Play calls `run(instanceId, serverAddress)` with `serverAddress` from the bootstrap payload. If the
   "一键进服" setting is off, please ignore the address on the backend side (the frontend always passes it).
