<script setup lang="ts">
/** 活动与赛季: events from GET /v1/summary with live countdowns. Ended ones are hidden. */
import { computed } from 'vue'

import { formatGalaxyDateTime } from '@/galaxy/announcements'
import { galaxyEventTiming } from '@/galaxy/countdown'
import type { GalaxyEvent, GalaxyEventKind } from '@/galaxy/server-api'

const props = defineProps<{
    events: GalaxyEvent[]
    now: Date
}>()

const KIND_LABEL: Record<GalaxyEventKind, string> = {
    season: '赛季',
    activity: '活动',
    maintenance: '维护',
}

const items = computed(() =>
    props.events
        .map((event) => ({ event, timing: galaxyEventTiming(event.start, event.end, props.now) }))
        .filter(({ timing }) => timing.phase !== 'ended')
        // Running first, then upcoming by start time.
        .sort((a, b) => {
            const rank = (phase: string) => (phase === 'running' ? 0 : phase === 'upcoming' ? 1 : 2)
            return (
                rank(a.timing.phase) - rank(b.timing.phase) ||
                (a.event.start ?? '').localeCompare(b.event.start ?? '')
            )
        }),
)

function range(event: GalaxyEvent) {
    const start = formatGalaxyDateTime(event.start, props.now)
    const end = formatGalaxyDateTime(event.end, props.now)
    if (start && end) return `${start} – ${end}`
    return start ? `${start} 开始` : end ? `${end} 结束` : ''
}
</script>

<template>
    <ul v-if="items.length" class="galaxy-events">
        <li v-for="{ event, timing } in items" :key="event.id" :class="`is-${event.kind}`">
            <div class="flex min-w-0 items-center gap-1.5">
                <span class="galaxy-events-kind">{{ KIND_LABEL[event.kind] }}</span>
                <span class="galaxy-events-title">{{ event.title }}</span>
            </div>
            <p v-if="event.description" class="galaxy-events-description">
                {{ event.description }}
            </p>
            <div class="galaxy-events-meta">
                <span
                    v-if="timing.text"
                    class="galaxy-events-countdown"
                    :class="`is-${timing.phase}`"
                    >{{ timing.text }}</span
                >
                <span class="galaxy-events-range">{{ range(event) }}</span>
            </div>
            <div
                v-if="timing.progress !== null"
                class="galaxy-events-bar"
                role="progressbar"
                aria-valuemin="0"
                aria-valuemax="100"
                :aria-valuenow="Math.round(timing.progress * 100)"
            >
                <span :style="{ width: `${(timing.progress * 100).toFixed(1)}%` }" />
            </div>
        </li>
    </ul>
</template>

<style scoped>
.galaxy-events {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    margin: 0;
    padding: 0;
    list-style: none;
}

.galaxy-events li {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    padding: 0.6rem 0.7rem;
    border-radius: 0.75rem;
    background: var(--surface-2);
}

.galaxy-events-kind {
    flex-shrink: 0;
    padding: 0.05rem 0.45rem;
    border-radius: 9999px;
    background: var(--color-brand-highlight);
    color: var(--color-brand);
    font-size: 0.7rem;
    font-weight: 700;
}

.is-maintenance .galaxy-events-kind {
    background: color-mix(in srgb, var(--color-orange) 14%, transparent);
    color: var(--color-orange);
}

.galaxy-events-title {
    min-width: 0;
    overflow: hidden;
    color: var(--color-text-primary);
    font-size: 0.875rem;
    font-weight: 700;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.galaxy-events-description {
    display: -webkit-box;
    margin: 0;
    overflow: hidden;
    color: var(--color-text-default);
    font-size: 0.8rem;
    line-height: 1.45;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
}

.galaxy-events-meta {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0.2rem 0.6rem;
    font-size: 0.75rem;
}

.galaxy-events-countdown {
    color: var(--color-brand);
    font-weight: 700;
}

.galaxy-events-countdown.is-upcoming {
    color: var(--color-orange);
}

.galaxy-events-range {
    color: var(--color-text-tertiary);
}

.galaxy-events-bar {
    height: 0.3rem;
    overflow: hidden;
    border-radius: 9999px;
    background: var(--surface-5);
}

.galaxy-events-bar > span {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: var(--color-brand);
}
</style>
