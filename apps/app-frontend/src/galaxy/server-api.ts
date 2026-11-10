/**
 * Galaxy宝可梦 server data API (GalaxyLauncherApi plugin on the main server,
 * GALAXY_LAUNCHER_API.md at the repository root): typed, validated views of
 * `/v1/summary` and `/v1/player/{name}`. Requests go through the backend
 * command `plugin:galaxy|galaxy_api_get` (fixed paths only, 8 s timeout, 1 MB cap).
 *
 * Every parser is defensive: unknown fields are ignored, wrong types become
 * null, and the UI only shows what is present.
 */
import { GALAXY_PLAYER_NAME_PATTERN, galaxyApiGet } from '@/helpers/galaxy'

export type GalaxyEventKind = 'season' | 'activity' | 'maintenance'

export interface GalaxyEvent {
    id: string
    kind: GalaxyEventKind
    title: string
    description: string
    /** Local time 'YYYY-MM-DDTHH:mm(:ss)'. */
    start: string | null
    end: string | null
}

export interface GalaxyOnlinePlayer {
    name: string
    uuid: string | null
}

export interface GalaxySummary {
    generatedAt: string | null
    server: {
        name: string
        online: number
        max: number
        tps: number | null
        players: GalaxyOnlinePlayer[]
        maintenance: { active: boolean; message: string }
    }
    events: GalaxyEvent[]
}

export interface GalaxyPlayerReminder {
    id: string
    text: string
    command: string | null
}

export interface GalaxyPlayerPass {
    tier: number | null
    maxTier: number | null
    season: string | null
    seasonEnd: string | null
}

export interface GalaxyPlayerData {
    name: string
    /** false: never joined the main server. */
    known: boolean
    online: boolean
    firstJoined: string | null
    lastSeen: string | null
    money: number | null
    moneyName: string | null
    points: number | null
    level: number | null
    pass: GalaxyPlayerPass | null
    signedInToday: boolean | null
    signInStreak: number | null
    reminders: GalaxyPlayerReminder[]
}

type Json = Record<string, unknown>

function record(value: unknown): Json | null {
    return value && typeof value === 'object' && !Array.isArray(value) ? (value as Json) : null
}

function str(value: unknown, max = 500): string | null {
    if (typeof value !== 'string') return null
    return value.length > max ? value.slice(0, max) : value
}

function num(value: unknown): number | null {
    return typeof value === 'number' && Number.isFinite(value) ? value : null
}

function bool(value: unknown): boolean | null {
    return typeof value === 'boolean' ? value : null
}

const DATE_PATTERN = /^\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}(?::\d{2})?$/

function date(value: unknown): string | null {
    const text = str(value, 32)?.trim()
    return text && DATE_PATTERN.test(text) ? text : null
}

function parseJson(text: string): Json {
    const value = record(JSON.parse(text))
    if (!value) throw new Error('服务器数据格式不正确')
    return value
}

export function parseGalaxySummary(text: string): GalaxySummary {
    const root = parseJson(text)
    const server = record(root.server) ?? {}
    const maintenance = record(server.maintenance) ?? {}
    const players: GalaxyOnlinePlayer[] = []
    if (Array.isArray(server.players)) {
        for (const entry of server.players.slice(0, 50)) {
            const item = record(entry)
            const name = str(item?.name, 16)
            if (!name || !GALAXY_PLAYER_NAME_PATTERN.test(name)) continue
            players.push({ name, uuid: str(item?.uuid, 36) })
        }
    }
    const events: GalaxyEvent[] = []
    if (Array.isArray(root.events)) {
        for (const entry of root.events.slice(0, 20)) {
            const item = record(entry)
            const id = str(item?.id, 64)
            const title = str(item?.title, 100)?.trim()
            if (!item || !id || !title) continue
            const kind =
                item.kind === 'season' || item.kind === 'maintenance' ? item.kind : 'activity'
            events.push({
                id,
                kind,
                title,
                description: str(item.description, 300) ?? '',
                start: date(item.start),
                end: date(item.end),
            })
        }
    }
    return {
        generatedAt: date(root.generatedAt),
        server: {
            name: str(server.name, 64) ?? '',
            online: Math.max(0, num(server.online) ?? 0),
            max: Math.max(0, num(server.max) ?? 0),
            tps: num(server.tps),
            players,
            maintenance: {
                active: maintenance.active === true,
                message: str(maintenance.message, 300) ?? '',
            },
        },
        events,
    }
}

export function parseGalaxyPlayer(text: string, requested: string): GalaxyPlayerData {
    const root = parseJson(text)
    const pass = record(root.pass)
    const reminders: GalaxyPlayerReminder[] = []
    if (Array.isArray(root.reminders)) {
        for (const entry of root.reminders.slice(0, 10)) {
            const item = record(entry)
            const id = str(item?.id, 64)
            const reminderText = str(item?.text, 200)
            if (!id || !reminderText) continue
            const command = str(item?.command, 100)
            reminders.push({
                id,
                text: reminderText,
                command: command?.startsWith('/') ? command : null,
            })
        }
    }
    return {
        name: str(root.name, 16) ?? requested,
        known: root.known === true,
        online: root.online === true,
        firstJoined: date(root.firstJoined),
        lastSeen: date(root.lastSeen),
        money: num(root.money),
        moneyName: str(root.moneyName, 20),
        points: num(root.points),
        level: num(root.level),
        pass: pass
            ? {
                  tier: num(pass.tier),
                  maxTier: num(pass.maxTier),
                  season: str(pass.season, 20),
                  seasonEnd: date(pass.seasonEnd),
              }
            : null,
        signedInToday: bool(root.signedInToday),
        signInStreak: num(root.signInStreak),
        reminders,
    }
}

export async function fetchGalaxySummary(): Promise<GalaxySummary> {
    return parseGalaxySummary(await galaxyApiGet('/v1/summary'))
}

export async function fetchGalaxyPlayer(name: string): Promise<GalaxyPlayerData> {
    if (!GALAXY_PLAYER_NAME_PATTERN.test(name)) throw new Error('游戏名格式不正确')
    return parseGalaxyPlayer(await galaxyApiGet(`/v1/player/${name}`), name)
}
