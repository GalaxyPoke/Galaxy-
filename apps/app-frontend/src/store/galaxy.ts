import { defineStore } from 'pinia'
import { computed, ref } from 'vue'

import { process_listener } from '@/helpers/events'
import {
    GALAXY_SERVER_ADDRESS,
    type GalaxyBootstrap,
    galaxyBootstrap,
    galaxyErrorText,
    galaxySyncApply,
    galaxySyncCheck,
    type GalaxySyncProgress,
    type GalaxySyncResult,
    type GalaxySyncSnapshot,
    galaxySyncStatus,
    listenGalaxySyncProgress,
} from '@/helpers/galaxy'
import { get_all as getAllProcesses } from '@/helpers/process.js'

/** Re-check the modpack when the window regains focus after this long without a check. */
const SYNC_IDLE_RECHECK_MS = 30 * 60 * 1000
/** A result this fresh (e.g. after a page reload) is reused instead of checking again. */
const SYNC_REUSE_MS = 5 * 60 * 1000
/** Backend text when another check / apply / launch holds the sync lock. */
const SYNC_BUSY_TEXT = '正在检查或更新整合包'

/**
 * Galaxy宝可梦: holds the `galaxy_bootstrap` payload (fixed instance id, server
 * address, game directory) and the state of the built-in modpack updater
 * (`galaxy_sync_*`, GALAXY_SYNC.md). `bootstrap()` runs once at app start;
 * later calls reuse the same promise unless `force` is set. Once the game is
 * found the modpack is checked, and a pending update is installed right away.
 */
export const useGalaxyStore = defineStore('galaxy', () => {
    const payload = ref<GalaxyBootstrap | null>(null)
    const loading = ref(false)
    const invokeError = ref<string | null>(null)
    let pending: Promise<GalaxyBootstrap | null> | null = null

    const instanceId = computed(() => payload.value?.instanceId ?? null)
    const serverAddress = computed(() => payload.value?.serverAddress || GALAXY_SERVER_ADDRESS)
    const gameFound = computed(() => payload.value?.gameFound ?? false)
    /** Chinese error text from the backend, or from a failed invoke. */
    const error = computed(() => payload.value?.error ?? invokeError.value)
    const ready = computed(() => !!payload.value && !!instanceId.value && gameFound.value)
    /** The built-in modpack updater is configured (true once the game is found). */
    const dreamingfish = computed(() => payload.value?.dreamingfish === true)
    /** Server data API base, null when this build has none (GALAXY_LAUNCHER_API.md). */
    const apiBase = computed(() => payload.value?.apiBase || null)

    async function bootstrap(force = false): Promise<GalaxyBootstrap | null> {
        if (pending && !force) return pending
        loading.value = true
        invokeError.value = null
        pending = galaxyBootstrap()
            .then((result) => {
                payload.value = result
                if (result.gameFound && result.instanceId && (force || !syncStarted)) {
                    syncStarted = true
                    void startupSync()
                }
                return result
            })
            .catch((err) => {
                console.error('[galaxy] galaxy_bootstrap failed', err)
                invokeError.value = `无法初始化游戏实例：${galaxyErrorText(err)}`
                return null
            })
            .finally(() => {
                loading.value = false
            })
        return pending
    }

    // ---- built-in modpack updater ---------------------------------------------
    const syncResult = ref<GalaxySyncResult | null>(null)
    const syncProgress = ref<GalaxySyncProgress | null>(null)
    /** Set while a check ('check') or a download + install ('apply') runs. */
    const syncRunning = ref<'check' | 'apply' | null>(null)
    /** The running check ignores the hash cache (修复游戏文件). */
    const syncForced = ref(false)
    /** Bumped when a sync changed launcher-update/*: App.vue re-checks the launcher update. */
    const launcherUpdateTick = ref(0)
    /** Set when an update was installed; the home page shows a toast once and clears it. */
    const appliedNotice = ref<GalaxySyncResult | null>(null)
    /** Set by the home page while it launches the game (no idle re-check then). */
    const launchInProgress = ref(false)
    let syncStarted = false
    let syncPromise: Promise<GalaxySyncResult | null> | null = null
    let lastSyncAt = 0
    let listening: Promise<void> | null = null

    const syncBusy = computed(() => syncRunning.value !== null)
    /** Offline is fine (the local pack is used); a blocked or failed sync disables play. */
    const syncBlocksPlay = computed(
        () =>
            !syncRunning.value &&
            (syncResult.value?.status === 'blocked' || syncResult.value?.status === 'error'),
    )

    function ensureListeners(): Promise<void> {
        listening ??= Promise.all([
            listenGalaxySyncProgress((progress) => {
                if (syncRunning.value) syncProgress.value = progress
            }),
            // An update that was found while the game ran is installed once it exits.
            process_listener((e: { event: string; instance_id: string }) => {
                if (e.event !== 'finished' || e.instance_id !== instanceId.value) return
                if (syncResult.value?.status === 'updateAvailable') void sync()
            }),
        ])
            .then(() => undefined)
            .catch((err) => {
                console.warn('[galaxy] failed to listen for modpack sync events', err)
            })
        return listening
    }

    function failedResult(message: string): GalaxySyncResult {
        return {
            status: 'error',
            code: 'INVOKE_FAILED',
            message,
            localVersion: syncResult.value?.localVersion ?? null,
            remoteVersion: syncResult.value?.remoteVersion ?? null,
            releaseId: null,
            sequence: null,
            changelog: null,
            releasedAt: null,
            filesToDownload: 0,
            bytesToDownload: 0,
            filesToRemove: 0,
            hashedFiles: 0,
            applied: false,
            filesInstalled: 0,
            filesBackedUp: 0,
            backupDir: null,
            launcherUpdateChanged: false,
            history: syncResult.value?.history ?? [],
            checkedAt: new Date().toISOString(),
        }
    }

    async function gameRunning(): Promise<boolean> {
        const list = await getAllProcesses().catch(() => [])
        return Array.isArray(list) && list.length > 0
    }

    /**
     * Checks the modpack (`force`: re-hash every file) and, unless `apply` is
     * false or the game is running, installs a pending update right away.
     * Concurrent calls share one run. Resolves with the final result.
     */
    function sync(
        options: { force?: boolean; apply?: boolean } = {},
    ): Promise<GalaxySyncResult | null> {
        if (syncPromise) return syncPromise
        const force = options.force ?? false
        const apply = options.apply ?? true
        syncPromise = (async () => {
            await ensureListeners()
            syncForced.value = force
            syncRunning.value = 'check'
            syncProgress.value = {
                phase: 'checking',
                doneFiles: 0,
                totalFiles: 0,
                doneBytes: 0,
                totalBytes: 0,
                currentFile: null,
                message: null,
            }
            try {
                let result = await galaxySyncCheck(force)
                syncResult.value = result
                if (result.status === 'updateAvailable' && apply && !(await gameRunning())) {
                    syncRunning.value = 'apply'
                    result = await galaxySyncApply()
                    syncResult.value = result
                    if (result.applied) appliedNotice.value = result
                    if (result.launcherUpdateChanged) launcherUpdateTick.value += 1
                }
                lastSyncAt = Date.now()
                return result
            } catch (err) {
                const message = galaxyErrorText(err)
                // Another check / launch holds the lock: keep the last result.
                if (message.includes(SYNC_BUSY_TEXT)) return syncResult.value
                console.error('[galaxy] modpack sync failed', err)
                syncResult.value = failedResult(message)
                lastSyncAt = Date.now()
                return syncResult.value
            } finally {
                syncRunning.value = null
                syncProgress.value = null
                syncForced.value = false
            }
        })().finally(() => {
            syncPromise = null
        })
        return syncPromise
    }

    /** After a page reload: follow a sync that is still running in the backend. */
    function follow(snapshot: GalaxySyncSnapshot): Promise<GalaxySyncResult | null> {
        const phase = snapshot.progress?.phase
        syncRunning.value = phase === 'downloading' || phase === 'applying' ? 'apply' : 'check'
        syncProgress.value = snapshot.progress
        syncPromise = (async () => {
            for (;;) {
                await new Promise((resolve) => setTimeout(resolve, 1000))
                const next = await galaxySyncStatus().catch(() => null)
                if (!next?.busy) {
                    if (next?.lastResult) syncResult.value = next.lastResult
                    lastSyncAt = Date.now()
                    return syncResult.value
                }
            }
        })().finally(() => {
            syncRunning.value = null
            syncProgress.value = null
            syncPromise = null
        })
        return syncPromise
    }

    async function startupSync(): Promise<void> {
        await ensureListeners()
        const snapshot = await galaxySyncStatus().catch(() => null)
        if (snapshot?.busy) {
            await follow(snapshot)
            return
        }
        const last = snapshot?.lastResult
        if (
            last?.status === 'upToDate' &&
            Date.now() - Date.parse(last.checkedAt) < SYNC_REUSE_MS
        ) {
            syncResult.value = last
            lastSyncAt = Date.parse(last.checkedAt)
            return
        }
        await sync()
    }

    function recheckIfIdle() {
        if (!ready.value || syncPromise || launchInProgress.value) return
        if (Date.now() - lastSyncAt < SYNC_IDLE_RECHECK_MS) return
        void sync()
    }

    if (typeof window !== 'undefined') {
        window.addEventListener('focus', recheckIfIdle)
        document.addEventListener('visibilitychange', () => {
            if (document.visibilityState === 'visible') recheckIfIdle()
        })
    }

    return {
        payload,
        loading,
        instanceId,
        serverAddress,
        gameFound,
        error,
        ready,
        dreamingfish,
        apiBase,
        bootstrap,
        syncResult,
        syncProgress,
        syncRunning,
        syncForced,
        syncBusy,
        syncBlocksPlay,
        launcherUpdateTick,
        appliedNotice,
        launchInProgress,
        sync,
    }
})
