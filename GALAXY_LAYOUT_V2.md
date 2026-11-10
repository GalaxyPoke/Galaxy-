# Galaxy宝可梦 layout v2 — "server lobby" shell

Goal (user, 2026-10-31): the launcher must feel built *for this one server*, not "Axolotl with a server added".
Keep the visual language (Modrinth light theme, white surfaces, rounded cards, green accent `--color-brand`,
filled buttons `--color-brand-fill` #1bd96a with black text, Inter font, soft shadows), but change the layout so it
no longer looks like Axolotl/Modrinth App.

All user-facing text is Chinese. No invented facts (no fake QQ group, website, player counts) — only data we have.

## What to remove from the Axolotl shell
- Left icon nav rail (`.app-grid-navbar`, NavRail) — gone.
- Right sidebar (`.app-sidebar`, "当前游玩账号", collapse handle) — gone on every page.
  Pages that teleport into `#sidebar-teleport-target` / `#sidebar-default-teleport-target` must keep working:
  keep those targets mounted (e.g. hidden container) or move the instance page's teleported content inline.
- Titlebar breadcrumbs and back/forward buttons.

## New shell
```
┌──────────────────────────────────────────────────────────────────────────────┐
│ [logo] Galaxy宝可梦 │  主页  游戏  皮肤  设置  │        🔔 ⬇  [● 游戏运行中 ■] [头像 Galaxy_Test ▾] ─ □ ✕ │  header 3.5rem, drag region
├──────────────────────────────────────────────────────────────────────────────┤
│                                 page content (full width)                    │
└──────────────────────────────────────────────────────────────────────────────┘
```
- Header: one row, `bg-surface-4` (white) with a 1px bottom border `--surface-5`; whole bar is `data-tauri-drag-region`,
  interactive children get `data-tauri-drag-region-exclude`.
  - Left: GalaxyLogo (28px) + wordmark "Galaxy宝可梦" (font-bold).
  - Tabs: 主页 `/`, 游戏 (`galaxyInstancePath`, disabled until `galaxy.instanceId`), 皮肤 `/skins`, 设置 `/settings`.
    Horizontal pill tabs with an animated green pill behind the active tab (active text uses `--color-brand-fill-contrast`
    on `--color-brand-fill`, or brand text on a light-green highlight — pick what reads best). Keep the
    `data-onboarding-id` values `nav-game`, `nav-skins`, `nav-settings` (and home) on the new tabs.
  - Right: existing AppActionBar (notifications, downloads, backups, offline, running-process pill) +
    **player chip**: Avatar (skin head) + current name; clicking opens a PopoutMenu containing the existing AccountsCard
    (switch name / add name / copy UUID). No account → chip says "设置游戏名" and opens the name modal.
    Then WindowControls.
- Content: `.app-contents` spans full width below the header, no rounded top-left inset, background `--surface-2`/`--color-bg`.
- Update every consumer of `--left-bar-width` / `--right-bar-width` / `--top-bar-height` (loading bar, notification panels'
  `has-sidebar`, `providePageContext.floatingActionBarOffsets`, studio-mode/console-fullscreen rules).

## 主页 (pages/Index.vue) — the server lobby
```
┌───────────────────────── hero banner (full width, ~260px) ─────────────────────────┐
│  green→teal gradient + subtle geometric pattern (pure CSS / inline SVG, no images)  │
│  [server icon 112px]  Galaxy宝可梦                                                  │
│                       MOTD (minecraft font)                                         │
│                       chips: ● 在线 12/100 · 38 毫秒 · 1.21.1 · Cobblemon 整合包     │
└─────────────────────────────────────────────────────────────────────────────────────┘
┌── 公告 (2/3 width) ───────────────────┐ ┌── 服务器 (1/3) ─────────────────────┐
│ announcement cards (existing data)    │ │ 地址 mc9.yunmaoc.com:27852 [复制]     │
│                                       │ │ 版本 / 在线玩家 sample names (if any) │
│                                       │ │ [刷新状态]                            │
│                                       │ ├── 快捷操作 ──────────────────────────┤
│                                       │ │ 游戏设置（内存/Java） → instance page  │
│                                       │ │ 打开游戏文件夹  查看日志  修复游戏文件 │
└───────────────────────────────────────┘ └──────────────────────────────────────┘
┌──────────────── play dock (sticky to bottom of the viewport, white, shadow) ─────────────────┐
│ [skin head] 以 Galaxy_Test 进入服务器  切换          梦鱼自动更新已启用          [ ▶ 开始游戏 ] │
└───────────────────────────────────────────────────────────────────────────────────────────────┘
```
- Play dock states: 开始游戏 (big, brand fill) / 启动中… N 秒 (disabled spinner) / 停止游戏 (red) / disabled with
  reason while bootstrapping or on error. Keep all existing logic (name modal before first launch, `run(id, serverAddress)`,
  process_listener, kill, error handling, retry bootstrap).
- "梦鱼自动更新已启用" only when `galaxy` bootstrap reports `dreamingfish: true` (add it to the store if missing).
- Quick actions reuse existing helpers: `showInstanceInFolder(id)` (helpers/utils.js), route to `/instance/<id>/logs`,
  instance settings (navigate to instance page; opening InstanceSettingsModal directly is a bonus), repair via
  `install_existing_instance(id, false)` with a confirm.
- Responsive down to the minimum window size (check tauri.conf.json minWidth/minHeight): columns stack.

## 游戏 page (pages/instance/Index.vue)
Must look right in the new full-width shell (no right sidebar). Keep tabs (内容/截图/日志) and header actions.
Light touch: page max-width container, consistent with home cards.

## Settings
Hide categories that make no sense for a single-server launcher: `ai`, `mcp`, `instance-sync`, `content-downloads`
(add to `GALAXY_HIDDEN_CATEGORY_IDS`). Settings page itself keeps its own left category list.

## Verification
Build `pnpm --filter @modrinth/app-frontend run build` (vue-tsc + vite) and take harness screenshots
(D:\GalaxyPokemon\_devtools\ui-harness: serve.mjs + headless Edge + cdp-shot.mjs) of `/`, `/instance/galaxy`,
`/instance/galaxy/logs`, `/skins`, `/settings`, plus the player-chip popout open, at 1280x800 and at the minimum window size.
The red error toasts in harness shots come from mocked IPC returning null — ignore those, but no new console exceptions.
