/**
 * Shared, app-wide announcement state for the home page and /announcements.
 * Loads once per session (again on demand via `reload`) and re-filters by
 * startAt / endAt every minute.
 */
import { computed, onBeforeUnmount, ref, shallowRef } from 'vue'

import {
    activeGalaxyAnnouncements,
    type GalaxyAnnouncement,
    sortGalaxyAnnouncements,
} from './announcements'
import {
    type GalaxyAnnouncementSource,
    loadGalaxyAnnouncements,
} from './announcement-source'
import { bundledGalaxyAnnouncements } from './announcements-bundled'

const items = shallowRef<GalaxyAnnouncement[]>(bundledGalaxyAnnouncements)
const source = ref<GalaxyAnnouncementSource>('bundled')
const error = ref<string | null>(null)
const loading = ref(false)
const now = ref(new Date())
let pending: Promise<void> | null = null
let loaded = false
let users = 0
let clock: ReturnType<typeof setInterval> | undefined

function load(force = false): Promise<void> {
    if (pending) return pending
    if (loaded && !force) return Promise.resolve()
    loading.value = true
    pending = loadGalaxyAnnouncements()
        .then((result) => {
            items.value = result.items
            source.value = result.source
            error.value = result.error ?? null
            loaded = true
        })
        .finally(() => {
            loading.value = false
            pending = null
        })
    return pending
}

export function useGalaxyAnnouncements() {
    users += 1
    if (!clock) {
        now.value = new Date()
        clock = setInterval(() => (now.value = new Date()), 60_000)
    }
    onBeforeUnmount(() => {
        users -= 1
        if (users === 0 && clock) {
            clearInterval(clock)
            clock = undefined
        }
    })
    void load()

    const visible = computed(() =>
        sortGalaxyAnnouncements(activeGalaxyAnnouncements(items.value, now.value)),
    )
    return {
        announcements: visible,
        source,
        error,
        loading,
        now,
        reload: () => load(true),
    }
}
