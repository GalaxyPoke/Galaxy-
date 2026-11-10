/** Countdown texts for events and the pass season ('3 天 4 小时', '25 分钟'). */
import { parseGalaxyDateTime } from './announcements'

/** Remaining time as '3 天 4 小时' / '5 小时 20 分' / '12 分钟' / '不到 1 分钟'. */
export function formatGalaxyDuration(ms: number): string {
    const minutes = Math.floor(Math.max(0, ms) / 60_000)
    if (minutes < 1) return '不到 1 分钟'
    const days = Math.floor(minutes / 1440)
    const hours = Math.floor((minutes % 1440) / 60)
    const rest = minutes % 60
    if (days > 0) return hours > 0 ? `${days} 天 ${hours} 小时` : `${days} 天`
    if (hours > 0) return rest > 0 ? `${hours} 小时 ${rest} 分` : `${hours} 小时`
    return `${rest} 分钟`
}

export type GalaxyEventPhase = 'upcoming' | 'running' | 'ended' | 'unknown'

export interface GalaxyEventTiming {
    phase: GalaxyEventPhase
    /** '还有 3 天开始' / '还剩 2 天 4 小时' / '已结束' / ''. */
    text: string
    /** 0..1 elapsed share while running and both ends are known. */
    progress: number | null
}

export function galaxyEventTiming(
    start: string | null,
    end: string | null,
    now: Date,
): GalaxyEventTiming {
    const time = now.getTime()
    const startAt = parseGalaxyDateTime(start)?.getTime() ?? null
    const endAt = parseGalaxyDateTime(end)?.getTime() ?? null
    if (startAt !== null && time < startAt) {
        return {
            phase: 'upcoming',
            text: `还有 ${formatGalaxyDuration(startAt - time)}开始`,
            progress: null,
        }
    }
    if (endAt !== null && time >= endAt) return { phase: 'ended', text: '已结束', progress: null }
    if (endAt !== null) {
        const progress =
            startAt !== null && endAt > startAt
                ? Math.min(1, Math.max(0, (time - startAt) / (endAt - startAt)))
                : null
        return { phase: 'running', text: `还剩 ${formatGalaxyDuration(endAt - time)}`, progress }
    }
    return startAt !== null
        ? { phase: 'running', text: '进行中', progress: null }
        : { phase: 'unknown', text: '', progress: null }
}
