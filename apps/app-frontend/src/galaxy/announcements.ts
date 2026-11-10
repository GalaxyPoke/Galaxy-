/**
 * Galaxy宝可梦 announcements: data model shared by the home page and the
 * /announcements page. Fields mirror the server's PlayerAnnouncement plugin
 * (plugins/PlayerAnnouncement/announcements.yml). Titles and content lines may
 * contain Minecraft '&x' / '§x' codes; render them with GalaxyMinecraftText.
 *
 * Where the list comes from: ./announcement-source.ts (remote slot) with the
 * bundled fallback in ./announcements-bundled.ts.
 */
import { stripMinecraftCodes } from './minecraft-format'

export interface GalaxyAnnouncementButton {
    /** Button label, may contain colour codes, e.g. '&a[打开活动中心]'. */
    text: string
    /** Plugin action, e.g. RUN_COMMAND / SUGGEST_COMMAND. Commands cannot run in the launcher. */
    action?: string | null
    /** e.g. '/activity' */
    value?: string | null
}

export interface GalaxyAnnouncement {
    id: string
    title: string
    /** Lines; colour codes allowed. */
    content: string[]
    /** Free text, e.g. 服务器公告 / 服务器规则 / 活动预告 / 新手指南 / 安全提醒. */
    category: string
    pinned: boolean
    mustRead: boolean
    /** Higher first (after pinned). */
    priority: number
    /** Local time 'YYYY-MM-DDTHH:mm'. */
    createdAt: string
    /** Hidden before this local time. */
    startAt?: string | null
    /** Hidden after this local time. */
    endAt?: string | null
    button?: GalaxyAnnouncementButton | null
}

/** Parses 'YYYY-MM-DD', 'YYYY-MM-DDTHH:mm' or '…:ss' as local time. */
export function parseGalaxyDateTime(value: string | null | undefined): Date | null {
    if (!value) return null
    const match = /^(\d{4})-(\d{2})-(\d{2})(?:[T ](\d{2}):(\d{2})(?::(\d{2}))?)?$/.exec(value.trim())
    if (!match) return null
    const [, y, mo, d, h = '0', mi = '0', s = '0'] = match
    const date = new Date(Number(y), Number(mo) - 1, Number(d), Number(h), Number(mi), Number(s))
    return Number.isNaN(date.getTime()) ? null : date
}

function timeOf(value: string | null | undefined) {
    return parseGalaxyDateTime(value)?.getTime() ?? 0
}

/** Pinned first, then priority (high → low), then newest first. */
export function sortGalaxyAnnouncements(list: readonly GalaxyAnnouncement[]): GalaxyAnnouncement[] {
    return [...list].sort(
        (a, b) =>
            Number(b.pinned) - Number(a.pinned) ||
            b.priority - a.priority ||
            timeOf(b.createdAt) - timeOf(a.createdAt),
    )
}

/** Drops announcements that have not started yet or have already ended. */
export function activeGalaxyAnnouncements(
    list: readonly GalaxyAnnouncement[],
    now: Date = new Date(),
): GalaxyAnnouncement[] {
    const time = now.getTime()
    return list.filter((item) => {
        const start = parseGalaxyDateTime(item.startAt)
        const end = parseGalaxyDateTime(item.endAt)
        if (start && time < start.getTime()) return false
        if (end && time > end.getTime()) return false
        return true
    })
}

function pad(value: number) {
    return String(value).padStart(2, '0')
}

/** '10月30日' (adds the year when it is not the current year). */
export function formatGalaxyDate(value: string | null | undefined, now: Date = new Date()) {
    const date = parseGalaxyDateTime(value)
    if (!date) return ''
    const day = `${date.getMonth() + 1}月${date.getDate()}日`
    return date.getFullYear() === now.getFullYear() ? day : `${date.getFullYear()}年${day}`
}

/** '10月30日 18:00' */
export function formatGalaxyDateTime(value: string | null | undefined, now: Date = new Date()) {
    const date = parseGalaxyDateTime(value)
    if (!date) return ''
    return `${formatGalaxyDate(value, now)} ${pad(date.getHours())}:${pad(date.getMinutes())}`
}

/**
 * The in-game command hinted by an announcement button, e.g. '/activity'.
 * The launcher cannot run server commands, so the UI shows "进服后输入 …".
 */
export function galaxyAnnouncementCommand(item: GalaxyAnnouncement): string | null {
    const button = item.button
    if (!button?.value) return null
    const action = (button.action ?? '').toUpperCase()
    if (action !== 'RUN_COMMAND' && action !== 'SUGGEST_COMMAND') return null
    const value = stripMinecraftCodes(button.value).trim()
    return value.startsWith('/') ? value : null
}

/** Button label without codes and the surrounding [ ]. */
export function galaxyAnnouncementButtonLabel(item: GalaxyAnnouncement): string {
    if (!item.button?.text) return ''
    return stripMinecraftCodes(item.button.text)
        .trim()
        .replace(/^\[(.*)\]$/, '$1')
        .trim()
}
