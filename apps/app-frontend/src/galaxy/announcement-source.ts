/**
 * Where the launcher gets its announcements from.
 *
 * ─── Remote source ────────────────────────────────────────────────────────────
 * By default the list comes from `/v1/announcements` of the server data API
 * (GalaxyLauncherApi, GALAXY_LAUNCHER_API.md), which serves the main server's
 * PlayerAnnouncement file. GALAXY_ANNOUNCEMENTS_URL overrides that address.
 * The response must be a JSON array of GalaxyAnnouncement objects (see
 * ./announcements.ts and "Announcements source" in GALAXY_CONTRACT.md); an
 * object `{ "announcements": [...] }` is accepted too.
 *
 * - No URL (no override, no API base) → the bundled list (./announcements-bundled.ts), no network.
 * - URL set → GET through the backend command `plugin:galaxy|galaxy_fetch_announcements`
 *   (the webview CSP only allows a fixed set of hosts; the command accepts only
 *   http/https, times out after 10 s and caps the body at 1 MB). Entries are
 *   validated one by one and bad ones are dropped. The last good result is
 *   cached in localStorage; on failure the cache is used, then the bundled list.
 */
import { invoke } from '@tauri-apps/api/core'

import { useGalaxyStore } from '@/store/galaxy'

import type { GalaxyAnnouncement, GalaxyAnnouncementButton } from './announcements'
import { bundledGalaxyAnnouncements } from './announcements-bundled'

export const GALAXY_ANNOUNCEMENTS_URL = ''

export type GalaxyAnnouncementSource = 'remote' | 'cache' | 'bundled'

export interface GalaxyAnnouncementsResult {
    items: GalaxyAnnouncement[]
    source: GalaxyAnnouncementSource
    error?: string
}

const CACHE_KEY = 'galaxy.announcements.cache.v1'
const TIMEOUT_MS = 10_000
const MAX_ITEMS = 200
const MAX_LINES = 60
const MAX_TEXT = 2000
const DATE_PATTERN = /^\d{4}-\d{2}-\d{2}(?:[T ]\d{2}:\d{2}(?::\d{2})?)?$/

function bundled(): GalaxyAnnouncement[] {
    return bundledGalaxyAnnouncements.map((item) => ({ ...item, content: [...item.content] }))
}

function pick(record: Record<string, unknown>, ...keys: string[]): unknown {
    for (const key of keys) if (record[key] !== undefined) return record[key]
    return undefined
}

function text(value: unknown, max = MAX_TEXT): string | null {
    if (typeof value !== 'string') return null
    return value.length > max ? value.slice(0, max) : value
}

function dateText(value: unknown): string | null {
    const result = text(value, 32)?.trim()
    return result && DATE_PATTERN.test(result) ? result : null
}

function toButton(value: unknown): GalaxyAnnouncementButton | null {
    if (!value || typeof value !== 'object') return null
    const record = value as Record<string, unknown>
    const label = text(record.text, 200)
    if (!label) return null
    return {
        text: label,
        action: text(record.action, 64),
        value: text(record.value, 256),
    }
}

/** Returns a clean announcement, or null when the entry is unusable. */
export function toGalaxyAnnouncement(value: unknown): GalaxyAnnouncement | null {
    if (!value || typeof value !== 'object' || Array.isArray(value)) return null
    const record = value as Record<string, unknown>
    if (record.enabled === false) return null

    const rawId = record.id
    const id =
        typeof rawId === 'number' && Number.isFinite(rawId)
            ? String(rawId)
            : text(rawId, 128)?.trim()
    const title = text(record.title, 300)?.trim()
    if (!id || !title) return null

    const rawContent = record.content
    let content: string[]
    if (typeof rawContent === 'string') content = rawContent.split(/\r?\n/)
    else if (Array.isArray(rawContent))
        content = rawContent.filter((line): line is string => typeof line === 'string')
    else if (rawContent === undefined || rawContent === null) content = []
    else return null
    content = content.slice(0, MAX_LINES).map((line) => text(line) ?? '')

    const createdAt = dateText(pick(record, 'createdAt', 'created-at', 'created_at'))
    if (!createdAt) return null

    const priority = Number(pick(record, 'priority') ?? 0)
    return {
        id,
        title,
        content,
        category: text(record.category, 40)?.trim() || '公告',
        pinned: record.pinned === true,
        mustRead: pick(record, 'mustRead', 'must-read', 'must_read') === true,
        priority: Number.isFinite(priority) ? priority : 0,
        createdAt,
        startAt: dateText(pick(record, 'startAt', 'start-at', 'start_at')),
        endAt: dateText(pick(record, 'endAt', 'end-at', 'end_at')),
        button: toButton(record.button),
    }
}

/** Validates a parsed JSON payload; throws when it is not a list at all. */
export function parseGalaxyAnnouncements(payload: unknown): GalaxyAnnouncement[] {
    const wrapped =
        payload && typeof payload === 'object'
            ? (payload as { announcements?: unknown }).announcements
            : undefined
    const list: unknown[] | null = Array.isArray(payload)
        ? payload
        : Array.isArray(wrapped)
          ? wrapped
          : null
    if (!list) throw new Error('公告数据格式不正确')
    const seen = new Set<string>()
    const items: GalaxyAnnouncement[] = []
    for (const entry of list.slice(0, MAX_ITEMS)) {
        const item = toGalaxyAnnouncement(entry)
        if (!item || seen.has(item.id)) continue
        seen.add(item.id)
        items.push(item)
    }
    return items
}

function readCache(url: string): GalaxyAnnouncement[] | null {
    try {
        const raw = localStorage.getItem(CACHE_KEY)
        if (!raw) return null
        const cached = JSON.parse(raw) as { url?: unknown; items?: unknown }
        if (cached.url !== url) return null
        return parseGalaxyAnnouncements(cached.items)
    } catch {
        return null
    }
}

function writeCache(url: string, items: GalaxyAnnouncement[]) {
    try {
        localStorage.setItem(CACHE_KEY, JSON.stringify({ url, fetchedAt: Date.now(), items }))
    } catch {
        // Storage full or unavailable: the next start just fetches again.
    }
}

async function fetchRemote(url: string): Promise<GalaxyAnnouncement[]> {
    let timer: ReturnType<typeof setTimeout> | undefined
    const timeout = new Promise<never>((_, reject) => {
        timer = setTimeout(() => reject(new Error('获取公告超时')), TIMEOUT_MS + 500)
    })
    try {
        const body = await Promise.race([
            invoke<string>('plugin:galaxy|galaxy_fetch_announcements', { url }),
            timeout,
        ])
        if (typeof body !== 'string') throw new Error('公告服务器没有返回内容')
        return parseGalaxyAnnouncements(JSON.parse(body))
    } finally {
        if (timer) clearTimeout(timer)
    }
}

function errorText(error: unknown) {
    if (typeof error === 'string') return error
    if (error instanceof Error) return error.message
    if (error && typeof error === 'object' && 'message' in error)
        return String((error as { message: unknown }).message)
    return String(error)
}

/**
 * GALAXY_ANNOUNCEMENTS_URL when set, otherwise `/v1/announcements` of the
 * server data API (galaxy store `apiBase`, GALAXY_LAUNCHER_API.md), which
 * serves the main server's PlayerAnnouncement list live.
 */
async function announcementsUrl(): Promise<string> {
    const fixed = GALAXY_ANNOUNCEMENTS_URL.trim()
    if (fixed) return fixed
    try {
        const galaxy = useGalaxyStore()
        if (!galaxy.payload) await galaxy.bootstrap()
        return galaxy.apiBase ? `${galaxy.apiBase}/v1/announcements` : ''
    } catch {
        return ''
    }
}

export async function loadGalaxyAnnouncements(): Promise<GalaxyAnnouncementsResult> {
    const url = await announcementsUrl()
    if (!url) return { items: bundled(), source: 'bundled' }
    try {
        const items = await fetchRemote(url)
        writeCache(url, items)
        return { items, source: 'remote' }
    } catch (error) {
        const message = errorText(error)
        console.warn('[galaxy] failed to load announcements', error)
        const cached = readCache(url)
        if (cached) return { items: cached, source: 'cache', error: message }
        return { items: bundled(), source: 'bundled', error: message }
    }
}
