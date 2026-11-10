/**
 * Shared live data from the server data API for the home page: the summary
 * (online players, maintenance, events) is polled every 30 s while a page
 * uses it and the window is visible; player data is cached per name for 60 s.
 * Without an API base (galaxy store `apiBase`) everything stays 'disabled'.
 */
import { computed, onBeforeUnmount, type Ref, ref, shallowRef, watch } from 'vue'

import { galaxyErrorText } from '@/helpers/galaxy'
import { useGalaxyStore } from '@/store/galaxy'

import {
    fetchGalaxyPlayer,
    fetchGalaxySummary,
    type GalaxyPlayerData,
    type GalaxySummary,
} from './server-api'

export type GalaxyApiState = 'disabled' | 'loading' | 'ok' | 'error'

const SUMMARY_POLL_MS = 30_000
const PLAYER_TTL_MS = 60_000

const summary = shallowRef<GalaxySummary | null>(null)
const summaryState = ref<GalaxyApiState>('loading')
const summaryError = ref<string | null>(null)
let summaryPending: Promise<void> | null = null
let summaryUsers = 0
let pollTimer: ReturnType<typeof setInterval> | undefined

async function apiAvailable(): Promise<boolean> {
    const galaxy = useGalaxyStore()
    if (!galaxy.payload) await galaxy.bootstrap()
    return !!galaxy.apiBase
}

function refreshSummary(): Promise<void> {
    summaryPending ??= (async () => {
        if (!(await apiAvailable())) {
            summary.value = null
            summaryState.value = 'disabled'
            return
        }
        if (!summary.value) summaryState.value = 'loading'
        try {
            summary.value = await fetchGalaxySummary()
            summaryState.value = 'ok'
            summaryError.value = null
        } catch (error) {
            console.warn('[galaxy] failed to load the server summary', error)
            summary.value = null
            summaryState.value = 'error'
            summaryError.value = galaxyErrorText(error)
        }
    })().finally(() => {
        summaryPending = null
    })
    return summaryPending
}

function onVisible() {
    if (document.visibilityState === 'visible' && summaryUsers > 0) void refreshSummary()
}

export function useGalaxyServerSummary() {
    summaryUsers += 1
    if (summaryUsers === 1) {
        void refreshSummary()
        pollTimer = setInterval(() => {
            if (document.visibilityState === 'visible') void refreshSummary()
        }, SUMMARY_POLL_MS)
        document.addEventListener('visibilitychange', onVisible)
    }
    onBeforeUnmount(() => {
        summaryUsers -= 1
        if (summaryUsers === 0) {
            if (pollTimer) clearInterval(pollTimer)
            pollTimer = undefined
            document.removeEventListener('visibilitychange', onVisible)
        }
    })
    return {
        summary,
        state: summaryState,
        error: summaryError,
        refresh: refreshSummary,
    }
}

interface PlayerEntry {
    data: GalaxyPlayerData
    at: number
}

const playerCache = new Map<string, PlayerEntry>()

/** Live data for `name` (the selected game name); reload with `refresh(true)`. */
export function useGalaxyPlayer(name: Ref<string | null>) {
    const player = shallowRef<GalaxyPlayerData | null>(null)
    const state = ref<GalaxyApiState>('loading')
    let generation = 0

    async function refresh(force = false) {
        const current = name.value
        const run = ++generation
        if (!current) {
            player.value = null
            state.value = 'ok'
            return
        }
        if (!(await apiAvailable())) {
            if (run === generation) {
                player.value = null
                state.value = 'disabled'
            }
            return
        }
        const key = current.toLowerCase()
        const cached = playerCache.get(key)
        if (!force && cached && Date.now() - cached.at < PLAYER_TTL_MS) {
            player.value = cached.data
            state.value = 'ok'
            return
        }
        if (!player.value || player.value.name.toLowerCase() !== key) state.value = 'loading'
        try {
            const data = await fetchGalaxyPlayer(current)
            playerCache.set(key, { data, at: Date.now() })
            if (run !== generation) return
            player.value = data
            state.value = 'ok'
        } catch (error) {
            console.warn('[galaxy] failed to load player data', error)
            if (run !== generation) return
            player.value = null
            state.value = 'error'
        }
    }

    watch(name, () => void refresh(), { immediate: true })
    return {
        player,
        state,
        known: computed(() => (state.value === 'ok' ? (player.value?.known ?? null) : null)),
        refresh,
    }
}
