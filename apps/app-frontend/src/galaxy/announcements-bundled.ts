/**
 * Announcements bundled with the launcher. Shown when no remote source is set
 * (GALAXY_ANNOUNCEMENTS_URL in ./announcement-source.ts is empty) or when it
 * cannot be reached and nothing is cached.
 *
 * Entries 1–5 are copied from the server's
 * plugins/PlayerAnnouncement/announcements.yml (codes kept as-is); the last
 * entry is a launcher-only guide. Replace this whole list when the plugin data
 * changes.
 */
import type { GalaxyAnnouncement } from './announcements'

export const bundledGalaxyAnnouncements: GalaxyAnnouncement[] = [
    {
        id: '1',
        title: '&b欢迎来到银河宝可梦',
        content: [
            '&f欢迎各位训练家！这里是银河宝可梦正式服。',
            '&7- 输入 &e/announcement &7随时打开公告栏',
            '&7- 输入 &e/activity &7查看正在进行的活动',
            '&7- 输入 &e/cw <宝可梦名> &7查询图鉴、刷新地点和进化方式',
            '&7- 每天记得 &e/signin &7签到领取奖励',
            '&a祝你旅途愉快，早日完成图鉴！',
        ],
        category: '服务器公告',
        pinned: true,
        mustRead: true,
        priority: 100,
        createdAt: '2026-10-30T18:00',
        button: { text: '&a[打开活动中心]', action: 'RUN_COMMAND', value: '/activity' },
    },
    {
        id: '2',
        title: '&c服务器规则',
        content: [
            '&f进服即视为同意以下规则，违者视情节警告、禁言或封禁：',
            '&71. 禁止使用外挂、透视、自动脚本等作弊客户端',
            '&72. 禁止利用 BUG 刷物品、刷宝可梦，发现请及时反馈',
            '&73. 禁止破坏、偷窃他人建筑和领地',
            '&74. 禁止辱骂、刷屏、发布广告或引战言论',
            '&75. 交易请当面完成，私下现实交易出现纠纷管理不予处理',
            '&e遇到违规行为请用 /feedback 提交，附上截图更快处理。',
        ],
        category: '服务器规则',
        pinned: true,
        mustRead: true,
        priority: 90,
        createdAt: '2026-10-30T18:00',
        button: { text: '&e[我要反馈]', action: 'RUN_COMMAND', value: '/feedback' },
    },
    {
        id: '3',
        title: '&6万圣节捕捉狂欢开启',
        content: [
            '&f10月31日 0:00 至 11月2日 23:59 举办万圣节活动！',
            '&7- 幽灵系宝可梦出没更频繁',
            '&7- 完成活动任务可领取万圣节限定奖励',
            '&e活动详情请打开活动中心查看。',
        ],
        category: '活动预告',
        pinned: false,
        mustRead: false,
        priority: 80,
        createdAt: '2026-10-30T18:00',
        startAt: '2026-10-30T18:00',
        endAt: '2026-11-02T23:59',
        button: { text: '&6[查看活动]', action: 'RUN_COMMAND', value: '/activity' },
    },
    {
        id: '4',
        title: '&a新手常用指令',
        content: [
            '&e/announcement &7- 公告栏',
            '&e/activity &7- 活动中心',
            '&e/signin &7- 每日签到',
            '&e/cw <宝可梦名> &7- 宝可梦图鉴',
            '&e/plg &7- 公会',
            '&e/plw &7- 玩家地标',
            '&e/bingo &7- 宾果挑战',
            '&e/feedback &7- 提交反馈与建议',
        ],
        category: '新手指南',
        pinned: false,
        mustRead: false,
        priority: 50,
        createdAt: '2026-10-30T18:00',
        button: { text: '&a[查询图鉴]', action: 'SUGGEST_COMMAND', value: '/cw' },
    },
    {
        id: '5',
        title: '&e交易安全提醒',
        content: [
            '&f和其他玩家交易时请注意：',
            '&7- 交易前确认对方的宝可梦个体、性格和特性',
            '&7- 不要相信"先给我，等下再还"之类的说法',
            '&7- 管理员不会私聊索要你的宝可梦、物品或账号密码',
            '&e被骗请第一时间截图并通过 /feedback 提交。',
        ],
        category: '安全提醒',
        pinned: false,
        mustRead: false,
        priority: 40,
        createdAt: '2026-10-30T18:00',
    },
    {
        id: 'launcher-player-name',
        title: '第一次进服：设置你的游戏名',
        content: [
            '本服使用离线账户。第一次开始游戏时，启动器会让你填写游戏名（3–16 位英文字母、数字或下划线）。',
            '游戏名就是你在服务器里的身份，换名字就等于换成另一个玩家，请记好自己的名字。',
            '之后可以点右上角的头像切换或添加游戏名。',
        ],
        category: '新手指南',
        pinned: false,
        mustRead: false,
        priority: 45,
        createdAt: '2026-10-29T12:00',
    },
]
