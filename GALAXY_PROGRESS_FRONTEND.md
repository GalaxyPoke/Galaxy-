# 前端进度日志（Galaxy宝可梦）

- [x] routes：index.ts / utility.ts / instance.ts 已精简（首页、设置、皮肤、/instance/:id 的 Mods/ModsFilter/截图/日志），其它路由表文件保留在磁盘上但未注册；未知路径重定向到 `/`。
- [x] 品牌 logo：`src/assets/galaxy-logo.svg`（复制自 branding/icon.svg）；新增 `components/ui/GalaxyLogo.vue`，删除 `AxolotlLogo.vue`；AppTitlebar / OnboardingWelcome / SplashScreen 改用 Galaxy 图标与文字。
- [x] 实例页 `pages/instance/Index.vue`：标签只剩 内容/截图/日志；去掉 导出整合包、升级实例、右键「添加内容」。
- [x] `InstanceSettingsModal.vue`：只保留 常规/窗口/Java/启动准备；`GeneralSettings.vue` 去掉 复制实例、删除实例。
- [x] 模组列表只读：packages/ui `ContentManagerContext.readOnly`（layout/ContentToolbar/ContentEmptyState 隐藏 浏览/添加、更新、删除、批量更新、切换版本）；Mods.vue 设 `readOnly: true`、`hasUpdateSupport: false`，去掉 解绑整合包/管理备份/原理图工坊 入口。
- [x] Galaxy 状态：`helpers/galaxy.ts`（galaxy_bootstrap / galaxy_set_player / galaxy_launch_blocked 封装 + 名字校验）、`store/galaxy.ts`（Pinia，bootstrap 只调一次，失败转中文错误）。
- [x] App.vue：导航只剩 首页 / 游戏(`/instance/<id>`) / 皮肤 + 设置；去掉 QuickInstanceSwitcher、新建实例、Modrinth 账户菜单与登录流程、RemoteAnnouncements、UpdateAnnouncementModal、PrivacyConsentModal、OfflineAccountRestrictionModal、MinecraftAuthErrorModal、AuthGrantFlowWaitModal、Mojang 连通性监视与横幅；更新检查经 `GALAXY_LAUNCHER_UPDATES_ENABLED=false` 关闭；setupApp 调 `galaxy.bootstrap()` 并监听 `galaxy_launch_blocked` → 通知「梦鱼更新器阻止了启动：<reason>」。
- [x] `store/error.js` 不再走反盗版/微软登录错误弹窗；`providers/app-update.ts` 默认 updatesEnabled=false；设置里隐藏 主页与导航 / 更新 / 联机 三个分区。
- [x] 快捷键只留 首页/皮肤/设置；新手引导主线只留 欢迎/皮肤/离线账户/设置。
- [x] AccountsCard.vue 重写为纯离线：名字输入（`^[A-Za-z0-9_]{3,16}$`，中文报错）→ `galaxy_set_player`；无 Microsoft / Yggdrasil。
- [x] 首页 `pages/Index.vue` 重写：服务器横幅（实例名 / 地址 / 在线人数 / 延迟 / MOTD，`fetchCachedServerStatus` + `autoToHTML`）、开始游戏（`run(id, serverAddress)`，启动中计时、停止）、无离线账户时弹名字框 → `galaxy_set_player` → 启动；gameFound=false / error 时显示中文错误卡片 + 重新检测。公告来自 `src/galaxy/announcements.ts`。
- [x] 品牌：config.ts（Galaxy宝可梦 / com.galaxypokemon.launcher / galaxypokemon / UA `GalaxyPokemon-Launcher/${version} (${os})`，官网/QQ/赞助/支持链接置空，新增 upstreamRepositoryUrl / upstreamModrinthSourceUrl）；AboutSettings 重写（Galaxy logo、版本、署名行、上游链接）；Settings 页脚 / Skins / 引导吉祥物换成 Galaxy logo；theme 初始强调色 `custom:#b5332a`。
- [x] 文案：apps/app-frontend 与 packages/ui 的 en-US / zh-CN locale 中 "Axolotl"/"Axolotl Launcher"/"Modrinth App" → Galaxy宝可梦（脚本 `_devtools/fe-rebrand-locales.mjs`，原文件备份在 `_devtools/locale-backup`）；新增 `app.navigation.game`（Game / 游戏）。i18n-check、brand-guard 均通过。
- [x] 收尾：Skins 页离线也能点「设置游戏名」；FeatureFlagSettings 隐私弹窗预览按钮在无注入时隐藏。GALAXY_TODO.md 已追加前端 → 后端请求。
- 验证：`vue-tsc --noEmit` 通过；`pnpm --filter @modrinth/app-frontend run build` → `✓ built`（日志 `_devtools/fe-build.log`）；brand-guard 通过；i18n-check 通过。
- 未做 / 待办：AppActionBar 的下载按钮仍跳 `/downloads`（会被重定向回首页）；Skins 页未登录提示文案仍是“登录 Minecraft 账户”；源码里的 defaultMessage 仍写 Axolotl（运行时用 locale JSON，`intl:extract` 会覆盖 en-US，需要时再改源码）；ErrorModal 的支持链接为空；未接远程公告；未跑 lint/vitest。

## 白 + 绿主题（Modrinth 配色）
- [x] 默认主题改为浅色、强调色改为现有的 `green` 选项（Modrinth 自带色板：浅色主题 `--color-brand` = #00af5c，深色主题 = #1bd96a）。
  - 前端：`store/theme.ts` 默认 `selectedTheme: 'light'`、`selectedAccentColor: 'green'`；`index.html` 初始 class 改为 `light-mode accent-green`（避免加载前闪粉/黑）。
  - 后端：`state/settings.rs` 的 Default 改为 `Theme::Light` / `AccentColor::Green`；新增迁移 `20261105120000_galaxy-theme-defaults.sql`（仅把仍是旧默认 dark + pink / custom:#b5332a 的行改为 light + green，用户自选的不动）；`downgrade-app-db.mjs` 的 REVERTIBLE_SCHEMA 已登记 `20261105120000: {}`，`downgrade-app-db.test.mjs` 通过。没有改已发布的 20261104120000 迁移（改它会让已有安装的 sqlx 校验和不匹配）。
- [x] 主按钮用 Modrinth 品牌绿 #1bd96a + 黑字：`variables.scss` 新增 `.light-mode.accent-green { --color-brand-fill: #1bd96a; --color-brand-fill-contrast: #000 }`；`ButtonFrame.vue`（type=colored + color=brand）与 `classes.scss` 的 `.btn-primary` 读取这两个变量，未设置时回落到 `--color-brand` / `--color-accent-contrast`，其它主题/强调色不变。链接、开关、焦点环、选中导航仍用 #00af5c（#1bd96a 在白底上看不清）。
- [x] 首页 `pages/Index.vue`：去掉品牌色径向渐变；卡片改为白底 + 细边框 + `--shadow-card`；在线状态改为绿点；置顶/标签徽章改为中性灰；地址 code 同样中性。
- [x] `SplashScreen.vue`：不再强制 `.dark` 和粉色渐变，跟随主题；深色立方体图只在深色主题显示。注意 Vue scoped 样式里 `:global(.a) .b` 会编译成 `.a {…}`，必须写成 `:global(.a .b)`（第一次就因此把整个页面隐藏了）。
- 验证：vue-tsc、前端 build、brand-guard、`cargo check -p theseus_gui` 均通过；release exe 已重建。
- 截图：这台 RDP 会话当前不绘制桌面（CopyFromScreen 全白、PrintWindow 全黑），真实窗口无法截图。改用 `_devtools/ui-harness`（node serve.mjs 用模拟 Tauri IPC 托管 dist，设置取自新装的 app.db；无头 Edge + cdp-shot.mjs 截图）。截图：`_devtools/theme-home-harness.png`、`theme-instance-harness.png`、`theme-settings-harness.png`（右下角报错气泡来自模拟 IPC，不是应用本身）。
- 遗留：浅色主题下选中的标签/设置侧栏项是 #00af5c 底白字（Modrinth 原样，对比度约 2.9:1）；若要更高对比度需另行调整。

## 布局 v2：服务器大厅（GALAXY_LAYOUT_V2.md）
- [x] 外壳（App.vue）：去掉左侧图标导航栏（NavRail/NavButton）和右侧「当前游玩账号」侧栏及折叠把手；`--top-bar-height` 改为 3.5rem，`--left-bar-width` / `--right-bar-width` 固定为 0（@modrinth/ui 共享组件仍会读取）；`.app-contents` 占满整个宽度，不再有左上圆角和内阴影；加载条、`floatingActionBarOffsets`（0/0）、通知面板（不再传 `has-sidebar`）、`hierarchicalSidebarAvailable=false` 都已同步。`#sidebar-teleport-target` / `#sidebar-default-teleport-target` 放进一个隐藏容器，仍旧往里传送的页面不会报错（Skins 页的「离线皮肤兼容性」说明因此不再显示）。
- [x] 顶栏 `components/ui/AppTitlebar.vue` 重写：Galaxy 图标 + 「Galaxy宝可梦」字标 | 主页 / 游戏 / 皮肤 / 设置 横向标签，带滑动的品牌绿胶囊（#1bd96a 底 + 黑字）。没有 `galaxy.instanceId` 时「游戏」不可点。保留 `data-onboarding-id` nav-home / nav-game / nav-skins / nav-settings。去掉面包屑和前进/后退。右侧依次是 McpConnectionStatus、AppActionBar、玩家胶囊、WindowControls；整条顶栏是拖动区域。
- [x] 新增 `components/ui/PlayerChip.vue`：显示皮肤头像和当前游戏名，点击后用 v-show 展开弹出层，里面是唯一的一份 `<AccountsCard ref="accounts" expanded>`（一直挂载，`provide('accountsCard')` 照常可用）。没有账户时显示「设置游戏名」，点击直接打开取名弹窗。胶囊带 `data-onboarding-id="offline-account-entry"`。App 还 `provide('galaxyOpenPlayerMenu')`，给首页的「切换」用。
- [x] AccountsCard：新增 `expanded` prop，并额外暴露 `selectedName` / `avatarUrl` / `hasAccounts`。
- [x] 首页 `pages/Index.vue` 改为服务器大厅：品牌绿到青色的渐变横幅（纯 CSS + 内联 SVG 菱形图案），包含服务器图标、名称、MOTD（放在深色底板上）、在线 / 延迟 / Minecraft 版本 / 加载器标签；下方左边 2/3 是公告，右边 1/3 是「服务器」卡（地址 + 复制、版本、在线人数、players.sample 玩家名、刷新状态）和「快捷操作」（游戏设置 → `/instance/<id>?settings=1` 自动打开 InstanceSettingsModal；打开游戏文件夹 `showInstanceInFolder`；查看日志 → `/logs`；修复游戏文件 → ConfirmModal → `install_existing_instance(id,false)`）；底部是吸底的游玩栏：头像、「以 X 进入服务器」+ 切换（或「设置游戏名」），中间在 `galaxy.dreamingfish` 为 true 时显示「梦鱼自动更新已启用」（准备中或不可用时改为显示原因），右边是 开始游戏 / 启动中… N 秒 / 停止游戏。启动、取名、process_listener、错误处理、重新检测的逻辑都没变。窗口宽度小于 960px 时两列改为上下排列。
- [x] `store/galaxy.ts` 新增 `dreamingfish` computed。公告文案「点击上方」改为「点击下方的」。
- [x] 实例页：外层加 `max-w-[76rem]` 居中容器，头部去掉为侧栏预留的 `pr-2`；支持 `?settings=1`，打开设置弹窗后会把这个参数去掉。
- [x] 设置：`GALAXY_HIDDEN_CATEGORY_IDS` 增加 ai / mcp / instance-sync / content-downloads。
- 验证：`pnpm --filter @modrinth/app-frontend run build`（vue-tsc + vite）通过。在 harness 里截了 `_devtools/v2-home.png`、`v2-home-min.png`（1100×700）、`v2-instance.png`、`v2-logs.png`、`v2-skins.png`、`v2-settings.png`、`v2-player-menu.png`，另有 `v2-home-min-scrolled.png`、`v2-home-settings-action.png`；改动前的基线是 `v2-baseline-*.png`。截图脚本是 `_devtools/ui-harness/shoot-v2.ps1` 和 `cdp-shot-size.mjs`（截图时隐藏了模拟 IPC 产生的报错气泡）。serve.mjs 新增了几个模拟：`lightweight_mode_frontend_ready`、`instance_sync_direct_links`，以及 players.sample。
- 遗留：`AppTitlebar.visual.spec.ts` 仍在测旧的面包屑 / 前进后退，需要更新或删除（这次没跑 vitest）；NavRail.vue、NavButton.vue、helpers/sidebar-state.ts 已无人引用，但还留在磁盘上；harness 里的日志标签页是空的（模拟 IPC 下日志组件的 Suspense 一直没完成，容器高度正常）；AppActionBar 的运行状态胶囊仍显示上游文案「没有运行中的实例」。
## 布局 v2 遗留修复（v3）
- [x] AppActionBar：没有游戏运行时不再显示运行状态胶囊；运行时显示「游戏运行中」（悬停标题是实例名，点击进实例页），停止 / 日志按钮和多进程下拉都保留。
- [x] AccountsCard：`expanded`（顶栏玩家弹出层）模式不再渲染手风琴标题行，只显示名字单选列表（复制 UUID / 删除）和「添加其他游戏名」；非 expanded 时仍是原来的 Accordion。组件仍只挂载一次。
- [x] 实例页 NavTabs 加 `galaxy-instance-tabs`，在 `.light-mode.accent-green` 下把 `--color-button-bg-selected` / `--color-button-text-selected` 改为 `--color-brand-fill` / `--color-brand-fill-contrast`（#1bd96a + 黑字，和顶栏一致）；其它主题不受影响。
- [x] Skins 页：「离线皮肤兼容性」（以及 Yggdrasil 的「第三方皮肤管理」）不再传送到隐藏的侧栏，改为皮肤列表上方的 Admonition。Galaxy 构建里能到达的页面（首页、实例页、皮肤、设置）已没有往 `#sidebar-*-teleport-target` 传送的内容；仍在传送的 Browse / project 页面没有注册路由。
- [x] `AppTitlebar.visual.spec.ts` 重写：4 个标签与 onboarding id、绿色胶囊跟随当前路由（#1bd96a / 黑字）、游戏标签禁用、空闲时隐藏运行胶囊、运行时「游戏运行中」+ 按钮在顶栏内、MCP 控件可点。本机没有 Playwright 自带的 Chromium，用临时配置 `launchOptions.channel = 'msedge'` 跑通 5/5（临时配置已删除）；直接用 vitest.config.ts 跑需要先 `pnpm exec playwright install chromium`。
- [ ] NavRail.vue / NavButton.vue / helpers/sidebar-state.ts 没删：NavRail.visual.spec.ts 和（已无人引用的）QuickInstanceSwitcher.vue 仍 import NavRail/NavButton，helpers/settings.test.ts 仍 import sidebar-state.ts。要删需要连同这些一起处理。
- 验证：前端 build 通过（`_devtools/fe-build-v3.log`）。截图 `_devtools/v3-home.png`、`v3-player-menu.png`、`v3-instance.png`、`v3-skins.png`、`v3-running.png`（脚本 `_devtools/ui-harness/shoot-v3.ps1`；serve.mjs 的运行中模拟需在地址加 `?running=1` 才启用，另外新增了 `instance_get_many` 模拟）。
## 公告页 + 游戏名弹窗（v4，GALAXY_ANNOUNCEMENTS_AND_NAME.md）
- [x] 公告数据模型 `src/galaxy/announcements.ts`（与 PlayerAnnouncement 插件字段一致：title/content[]/category/pinned/mustRead/priority/createdAt/startAt/endAt/button）；排序 置顶 → priority 降序 → createdAt 新到旧；按 startAt/endAt 过滤（每分钟重新判断）。
- [x] 颜色代码：`src/galaxy/minecraft-format.ts` 把 `&x` / `§x` 解析成片段，`components/ui/GalaxyMinecraftText.vue` 只用文本插值 + class 渲染（没有 v-html）。支持 0-9a-f、l/o/n/m/r，k 忽略。浅色主题下白色当正文色，黄 / 青 / 灰等换成白底上看得清的深色；深色主题用原版颜色。
- [x] 内置公告 `src/galaxy/announcements-bundled.ts`：照搬正式服 announcements.yml 的 5 条（保留颜色代码）+ 启动器自己的「第一次进服：设置你的游戏名」。
- [x] 远程数据源占位 `src/galaxy/announcement-source.ts`：`GALAXY_ANNOUNCEMENTS_URL = ''`。为空时直接用内置列表；填了以后通过新的后端命令 `plugin:galaxy|galaxy_fetch_announcements` 获取（webview 的 CSP connect-src 是固定白名单，没有放宽），逐条校验、丢掉坏条目，成功结果缓存到 localStorage（`galaxy.announcements.cache.v1`，和 URL 绑定），失败时依次退回缓存、内置列表。共享状态在 `src/galaxy/use-announcements.ts`。
- [x] 后端：`packages/app-lib/src/api/galaxy.rs` 新增 `fetch_announcements(url)`（只允许 http/https、不允许 URL 里带账号密码、10 秒超时、正文上限 1 MB、必须是 UTF-8，用 `configured_client()` 即应用的代理 / DNS 设置）；`apps/app/src/api/galaxy.rs` 注册命令，`apps/app/build.rs` 的 galaxy 命令列表已加上。
- [x] 首页公告栏：最多 3 条紧凑卡片（分类、置顶、必读徽章，标题，2 行预览，日期），标题行右侧「查看全部 (N) →」；点卡片跳到 `/announcements#announcement-<id>`，该卡自动展开、滚动到位并短暂高亮。
- [x] 新页面 `pages/Announcements.vue`（路由 `/announcements`，名字 `Announcements`，登记在 routes/home.ts）：返回主页、标题 + 条数、分类筛选（`?category=` 查询参数）、超过 4 行折叠（展开全文 / 收起）、「活动至 …」、按钮提示「打开活动中心：进服后输入 /activity」、空状态、底部来源说明。最大宽度 60rem。顶栏「主页」在 `/announcements` 下保持高亮。
- [x] 游戏名弹窗 `components/ui/GalaxyPlayerNameModal.vue` 替换了首页和 AccountsCard 里的两个旧弹窗：品牌渐变横幅 + Logo，大号 Steve 头像 + 大输入框、n/16 计数、名牌样式预览，实时规则清单（未输入时是灰点，通过 ✓ / 不通过 ✗；已保存过的名字提示可直接选用；只差大小写时警告会变成另一个玩家），离线账户说明，已有名字快捷选择，内联错误。Enter 提交，未通过校验时按钮禁用，打开时自动聚焦。首页 variant=play（「保存并开始游戏」，保存后刷新 accountsCard 再启动），AccountsCard variant=add（「保存」）。打开弹窗时会收起顶栏玩家弹出层。Steve 头像生成函数移到 `src/galaxy/skin-head.ts`。
- 验证：前端 build（vue-tsc + vite）通过；`cargo check -p theseus_gui --release` 通过（galaxy.rs 无新警告）。截图 `_devtools/v4-home.png`、`v4-announcements.png`、`v4-announcements-filter.png`（新手指南）、`v4-announcements-min.png`（1100×700）、`v4-name-first.png`、`v4-name-invalid.png`（ab-）、`v4-name-add.png`；脚本 `_devtools/ui-harness/shoot-v4.mjs`（用 node 写是为了让中文脚本不被 PowerShell 编码破坏）。serve.mjs 新增可选的 `?noaccount=1`（auth 返回没有账户）。截图期间没有新的控制台异常。
- 未做：远程接口本身（等插件有数据库 / API 后填 URL）；远程获取路径只在代码层面写好，没有对真实服务端联调；没跑 lint / vitest。
