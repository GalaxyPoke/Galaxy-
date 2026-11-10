<script setup lang="ts">
/**
 * 我的数据: the selected player's numbers from the main server
 * (GET /v1/player/{name}). Cells without a value are left out.
 */
import { computed } from 'vue'

import { formatGalaxyDateTime } from '@/galaxy/announcements'
import { galaxyEventTiming } from '@/galaxy/countdown'
import type { GalaxyPlayerData } from '@/galaxy/server-api'

const props = defineProps<{
    player: GalaxyPlayerData
    now: Date
}>()

interface Cell {
    label: string
    value: string
    hint?: string
}

function amount(value: number) {
    return value.toLocaleString('zh-CN', { maximumFractionDigits: 1 })
}

const cells = computed<Cell[]>(() => {
    const p = props.player
    const list: Cell[] = []
    if (p.money !== null) list.push({ label: p.moneyName || '金币', value: amount(p.money) })
    if (p.points !== null) list.push({ label: '点券', value: amount(p.points) })
    if (p.level !== null) list.push({ label: '训练师等级', value: `Lv.${p.level}` })
    if (p.pass && p.pass.tier !== null) {
        const timing = galaxyEventTiming(null, p.pass.seasonEnd, props.now)
        list.push({
            label: p.pass.season ? `通行证 ${p.pass.season}` : '通行证',
            value:
                p.pass.maxTier !== null
                    ? `${p.pass.tier}/${p.pass.maxTier} 级`
                    : `${p.pass.tier} 级`,
            hint: timing.phase === 'running' ? `赛季${timing.text}` : undefined,
        })
    }
    if (p.signedInToday !== null || p.signInStreak !== null) {
        list.push({
            label: '签到',
            value: p.signedInToday === null ? '—' : p.signedInToday ? '今日已签' : '今天未签',
            hint: p.signInStreak !== null ? `连续 ${p.signInStreak} 天` : undefined,
        })
    }
    return list
})

const presence = computed(() => {
    if (props.player.online) return '在线'
    const last = formatGalaxyDateTime(props.player.lastSeen, props.now)
    return last ? `上次在线 ${last}` : ''
})
</script>

<template>
    <div class="galaxy-stats">
        <div v-if="presence" class="galaxy-stats-presence" :class="{ 'is-online': player.online }">
            <span class="galaxy-stats-dot" aria-hidden="true" />
            {{ presence }}
        </div>
        <dl v-if="cells.length" class="galaxy-stats-grid">
            <div v-for="cell in cells" :key="cell.label" class="galaxy-stats-cell">
                <dt>{{ cell.label }}</dt>
                <dd>{{ cell.value }}</dd>
                <dd v-if="cell.hint" class="galaxy-stats-hint">{{ cell.hint }}</dd>
            </div>
        </dl>
        <p v-else class="galaxy-stats-empty">暂时读不到你的数据，进一次服务器后再来看看。</p>
        <ul v-if="player.reminders.length" class="galaxy-stats-reminders">
            <li v-for="reminder in player.reminders" :key="reminder.id">
                <span>{{ reminder.text }}</span>
                <span v-if="reminder.command" class="galaxy-stats-command"
                    >进服输入 <code>{{ reminder.command }}</code></span
                >
            </li>
        </ul>
    </div>
</template>

<style scoped>
.galaxy-stats {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
}

.galaxy-stats-presence {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    color: var(--color-text-tertiary);
    font-size: 0.8125rem;
}

.galaxy-stats-dot {
    width: 0.5rem;
    height: 0.5rem;
    border-radius: 9999px;
    background: var(--color-text-tertiary);
}

.galaxy-stats-presence.is-online {
    color: var(--color-brand);
    font-weight: 600;
}

.galaxy-stats-presence.is-online .galaxy-stats-dot {
    background: var(--color-brand);
    box-shadow: 0 0 0 3px var(--color-brand-highlight);
}

.galaxy-stats-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.5rem;
    margin: 0;
}

.galaxy-stats-cell {
    display: flex;
    min-width: 0;
    flex-direction: column;
    gap: 0.1rem;
    padding: 0.5rem 0.65rem;
    border-radius: 0.65rem;
    background: var(--surface-2);
}

.galaxy-stats-cell dt {
    overflow: hidden;
    color: var(--color-text-tertiary);
    font-size: 0.75rem;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.galaxy-stats-cell dd {
    margin: 0;
    overflow: hidden;
    color: var(--color-text-primary);
    font-size: 0.95rem;
    font-weight: 700;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.galaxy-stats-cell dd.galaxy-stats-hint {
    color: var(--color-text-tertiary);
    font-size: 0.7rem;
    font-weight: 500;
}

.galaxy-stats-empty {
    margin: 0;
    color: var(--color-text-tertiary);
    font-size: 0.8125rem;
}

.galaxy-stats-reminders {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    margin: 0;
    padding: 0;
    list-style: none;
}

.galaxy-stats-reminders li {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 0.25rem 0.75rem;
    padding: 0.45rem 0.65rem;
    border: 1px solid color-mix(in srgb, var(--color-orange) 35%, transparent);
    border-radius: 0.65rem;
    background: color-mix(in srgb, var(--color-orange) 8%, transparent);
    color: var(--color-text-primary);
    font-size: 0.8125rem;
    font-weight: 600;
}

.galaxy-stats-command {
    color: var(--color-text-tertiary);
    font-weight: 500;
}

.galaxy-stats-command code {
    padding: 0 0.3rem;
    border-radius: 0.35rem;
    background: var(--surface-2);
    color: var(--color-text-default);
}
</style>
