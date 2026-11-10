# Announcements page + player-name dialog (2026-11-01)

User requests:
1. "公告想查看全部，来个公告界面". Later the server's PlayerAnnouncement plugin will get a database and the
   launcher should read from it — **the user will build that part later; leave a clean, empty slot for it now.**
2. "输入游戏名的界面太简陋了" — redesign the player-name dialog.

All UI text Chinese. Keep layout v2 language (GALAXY_LAYOUT_V2.md): white cards, green brand, rounded, Inter.

## A. Announcement data model (shared by home + page)
Mirror the plugin's fields (D:\GalaxyPokemon\GalaxyPokemon正式服\plugins\PlayerAnnouncement\announcements.yml,
read-only — never write to the live server folder):
```ts
interface GalaxyAnnouncement {
  id: string
  title: string            // may contain Minecraft '&x' / '§x' colour + format codes
  content: string[]        // lines, same codes allowed
  category: string         // e.g. 服务器公告 / 服务器规则 / 活动预告 / 新手指南 / 安全提醒 (free text)
  pinned: boolean
  mustRead: boolean
  priority: number         // higher first (after pinned)
  createdAt: string        // ISO local datetime 'YYYY-MM-DDTHH:mm'
  startAt?: string | null  // hide before
  endAt?: string | null    // hide after; show "已结束" never — just hide
  button?: { text: string } | null   // display only; commands (RUN_COMMAND /activity) cannot run in the launcher —
                                     // show as a hint chip "进服后输入 /activity" when action is a command
}
```
- Sort: pinned → priority desc → createdAt desc. Filter by startAt/endAt against the current time.
- Colour codes: render SAFELY (parse into segments and render with Vue text interpolation + inline style/class; no v-html
  of untrusted strings). Support 0-9a-f colours, l bold, o italic, n underline, m strike, r reset; k (obfuscated) → plain.
  Light theme: map very light colours (&f white, &e yellow, &7 gray…) to readable darker tones on white cards.
- Bundled fallback: replace the 3 placeholder entries in `src/galaxy/announcements.ts` with the 5 real entries currently in
  the plugin's announcements.yml (convert faithfully, keep codes), plus keep a launcher-specific "第一次进服：设置你的游戏名"
  guide entry. Put the bundled list in its own module so it's easy to replace.

## B. Data source slot (empty for now)
- `src/galaxy/announcement-source.ts` exporting `loadGalaxyAnnouncements(): Promise<{ items, source: 'remote'|'cache'|'bundled', error? }>`.
- One constant `GALAXY_ANNOUNCEMENTS_URL = ''` (documented: set to the HTTP endpoint that serves the JSON array above once
  the plugin's database/API exists). When empty → return bundled immediately. When set → fetch (10 s timeout), validate the
  shape defensively (drop bad entries), cache the last good result in localStorage, fall back cache → bundled on failure.
- If the webview CSP blocks the fetch, do NOT loosen CSP broadly; instead add a tiny Rust command in the existing galaxy
  plugin (apps/app/src/api/galaxy.rs + packages/app-lib/src/api/galaxy.rs, register in build.rs commands list) that does a
  GET with the app's HTTP client and returns the body — only http/https, size cap 1 MB. Decide after checking
  apps/app/tauri.conf.json CSP. Either way the URL stays a constant, not user input.
- Document the JSON contract + where to set the URL in GALAXY_CONTRACT.md ("Announcements source").

## C. Home (pages/Index.vue)
- Announcement column: show at most 3 items (pinned first), each a compact card (category badge, 置顶/必读 badges, title,
  2-line clamped preview, date). Header row: "公告" + right-aligned link "查看全部 (N) →" to `/announcements`.

## D. Announcements page (new route `/announcements`, name `Announcements`, register in routes/home.ts or utility.ts)
- No new header tab. The 主页 tab should stay highlighted on /announcements (AppTitlebar `active` for home: `/` or
  `/announcements`). Page has a back link "← 返回主页".
- Top: title "服务器公告" + count; category filter chips (全部 + categories present in data, with counts).
- List: cards, pinned first. Each card: badges (category, 置顶, 必读), title (colour-rendered), date (createdAt; if endAt show
  "活动至 11月2日 23:59"), content lines. Long cards (> 4 lines) collapsed with "展开全文 / 收起". Button hint chip if any.
- Empty state for a filter with no items. Small footer note showing source: bundled → "公告来自启动器内置内容"; remote →
  "公告来自服务器"; cache → "暂时无法连接服务器，显示的是上次获取的公告".
- Max width ~60rem centred; works at the minimum window size 1100x700.

## E. Player-name dialog redesign
There are two name prompts today (pages/Index.vue name modal before first launch, and AccountsCard.vue
`playerNameModal`). Replace both with ONE new component (e.g. components/ui/GalaxyPlayerNameModal.vue) used by both,
keeping all behaviour: validateGalaxyPlayerName (^[A-Za-z0-9_]{3,16}$), galaxySetPlayer(name), refreshing the
accountsCard, then (from home) launching the game; AccountsCard's "添加其他游戏名" uses it without launching.
Design:
- Header banner strip in brand gradient with the Galaxy logo and "欢迎来到 Galaxy宝可梦" (first time) or "添加游戏名".
- Large skin-head avatar (Steve head as used elsewhere) next to a big input; live name preview "你在服务器里会显示为 <name>".
- Live rule checklist under the input with ✓/✗: 3–16 个字符 · 只用英文字母、数字和下划线 · (if the name equals an
  existing saved name) "已经保存过这个名字，可直接选用". Character counter "n/16".
- Short explanation: 离线账户、名字就是你在服务器里的身份、换名字等于换一个新玩家、请记好.
- If saved names exist, show them as quick-pick chips ("或选择已有的游戏名").
- Buttons: 取消 / primary (home: "保存并开始游戏", accounts: "保存"). Enter submits when valid; disabled until valid;
  error from galaxySetPlayer shown inline. Autofocus input. Works with keyboard.
- Keep data-onboarding-id="offline-account-entry" where it is now (chip/buttons), unaffected.

## Verify
- `. .\galaxy-env.ps1` then `pnpm --filter @modrinth/app-frontend run build` (vue-tsc + vite) passes.
- `cargo check -p theseus_gui --release` passes if you touched Rust.
- Harness screenshots (D:\GalaxyPokemon\_devtools\ui-harness: serve.mjs 4719, headless Edge CDP 9222, cdp-shot.mjs /
  cdp-shot-size.mjs; see shoot-v3.ps1) saved as D:\GalaxyPokemon\_devtools\v4-home.png, v4-announcements.png,
  v4-announcements-filter.png (a category selected), v4-announcements-min.png (1100x700), v4-name-first.png (no account,
  dialog from 开始游戏 — make the mock return no users via an opt-in URL flag, e.g. ?noaccount=1), v4-name-invalid.png
  (typed "ab-" showing failed rules), v4-name-add.png (from player chip → 添加其他游戏名). Look at each and iterate.
- Files UTF-8 no BOM, LF; scan touched files for mojibake. Stop only the node/msedge processes you started.
