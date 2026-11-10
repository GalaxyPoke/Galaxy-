/**
 * Galaxy宝可梦: typed wrappers for the backend `galaxy` Tauri plugin.
 * Contract: GALAXY_CONTRACT.md at the repository root.
 */
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

export const GALAXY_SERVER_ADDRESS = 'mc9.yunmaoc.com:27852'
export const GALAXY_INSTANCE_NAME = 'Galaxy宝可梦'
/** Offline player names accepted by the server (same rule as the backend). */
export const GALAXY_PLAYER_NAME_PATTERN = /^[A-Za-z0-9_]{3,16}$/

export interface GalaxyBootstrap {
    instanceId: string | null
    serverAddress: string
    gameRoot: string | null
    instanceDir: string | null
    gameFound: boolean
    /** The built-in modpack updater is configured (always true once the game is found). */
    dreamingfish: boolean
    javaPath: string | null
    /** Server data API base (GALAXY_LAUNCHER_API.md); null hides the features that need it. */
    apiBase?: string | null
    error: string | null
}

export interface GalaxyLaunchBlocked {
    instanceId: string | null
    reason: string
}

export async function galaxyBootstrap(): Promise<GalaxyBootstrap> {
    return await invoke<GalaxyBootstrap>('plugin:galaxy|galaxy_bootstrap')
}

export async function galaxySetPlayer(name: string): Promise<{ uuid: string }> {
    return await invoke<{ uuid: string }>('plugin:galaxy|galaxy_set_player', { name })
}

export interface GalaxyLauncherUpdate {
    /** null for builds without galaxy-launcher.json (self-update disabled). */
    currentVersion: string | null
    available: boolean
    version: string | null
    notes: string | null
    size: number | null
}

/** Checks whether the modpack sync has delivered a newer launcher build. */
export async function galaxyLauncherUpdateCheck(): Promise<GalaxyLauncherUpdate> {
    return await invoke<GalaxyLauncherUpdate>('plugin:galaxy|galaxy_launcher_update_check')
}

/** Verifies and stages the update; the launcher exits and restarts itself afterwards. */
export async function galaxyLauncherUpdateApply(): Promise<string> {
    return await invoke<string>('plugin:galaxy|galaxy_launcher_update_apply')
}

// ---- built-in modpack updater (packages/app-lib/src/api/galaxy_sync.rs) ----------

export type GalaxySyncStatus = 'upToDate' | 'updateAvailable' | 'offline' | 'blocked' | 'error'

export interface GalaxySyncHistoryEntry {
    releaseId: string
    sequence: number
    displayVersion: string | null
    changelog: string | null
    createdAt: string | null
}

export interface GalaxySyncResult {
    status: GalaxySyncStatus
    /** OFFLINE, SIGNATURE_INVALID, REPLAY, PATH_UNSAFE, NEEDS_NEWER_LAUNCHER, APPLY_FAILED, … */
    code: string | null
    message: string | null
    localVersion: string | null
    remoteVersion: string | null
    releaseId: string | null
    sequence: number | null
    changelog: string | null
    releasedAt: string | null
    filesToDownload: number
    bytesToDownload: number
    filesToRemove: number
    hashedFiles: number
    applied: boolean
    filesInstalled: number
    filesBackedUp: number
    backupDir: string | null
    /** launcher-update/* changed: re-run the launcher update check. */
    launcherUpdateChanged: boolean
    history: GalaxySyncHistoryEntry[]
    checkedAt: string
}

export type GalaxySyncPhase = 'checking' | 'hashing' | 'downloading' | 'applying' | 'done' | 'error'

export interface GalaxySyncProgress {
    phase: GalaxySyncPhase
    doneFiles: number
    totalFiles: number
    doneBytes: number
    totalBytes: number
    currentFile: string | null
    message: string | null
}

export interface GalaxySyncSnapshot {
    busy: boolean
    lastResult: GalaxySyncResult | null
    progress: GalaxySyncProgress | null
}

/** Fetches + verifies the signed release and compares it with the local files. */
export async function galaxySyncCheck(forceRehash = false): Promise<GalaxySyncResult> {
    return await invoke<GalaxySyncResult>('plugin:galaxy|galaxy_sync_check', { forceRehash })
}

/** Downloads and installs the pending update (refused while the game runs). */
export async function galaxySyncApply(): Promise<GalaxySyncResult> {
    return await invoke<GalaxySyncResult>('plugin:galaxy|galaxy_sync_apply')
}

/** Last result and progress (for page reloads). */
export async function galaxySyncStatus(): Promise<GalaxySyncSnapshot> {
    return await invoke<GalaxySyncSnapshot>('plugin:galaxy|galaxy_sync_status')
}

export async function listenGalaxySyncProgress(
    handler: (payload: GalaxySyncProgress) => void,
): Promise<UnlistenFn> {
    return await listen<GalaxySyncProgress>('galaxy_sync_progress', (event) =>
        handler(event.payload),
    )
}

export async function listenGalaxyLaunchBlocked(
    handler: (payload: GalaxyLaunchBlocked) => void,
): Promise<UnlistenFn> {
    return await listen<GalaxyLaunchBlocked>('galaxy_launch_blocked', (event) =>
        handler(event.payload),
    )
}

// ---- server data API (packages/app-lib/src/api/galaxy_api.rs, GALAXY_LAUNCHER_API.md) ----

/** GETs `/v1/summary`, `/v1/announcements`, `/v1/health` or `/v1/player/{name}`; returns the JSON text. */
export async function galaxyApiGet(path: string): Promise<string> {
    return await invoke<string>('plugin:galaxy|galaxy_api_get', { path })
}

export interface GalaxyCrashUploadFile {
    name: string
    content: string
}

export interface GalaxyCrashUploadInput {
    name: string | null
    summary: string
    modpackVersion: string | null
    memoryMb: number | null
    /** At most 4; the backend trims them to the server's 2 MB limit. */
    files: GalaxyCrashUploadFile[]
}

/** Uploads a crash report to the server; resolves with the report id. */
export async function galaxyCrashUpload(report: GalaxyCrashUploadInput): Promise<{ id: string }> {
    return await invoke<{ id: string }>('plugin:galaxy|galaxy_crash_upload', { report })
}

// ---- graphics presets (packages/app-lib/src/api/galaxy_graphics.rs) ----

export type GalaxyGraphicsPreset = 'low' | 'balanced' | 'high'

export interface GalaxyGraphicsState {
    /** null: the player's own settings. */
    preset: GalaxyGraphicsPreset | null
    optionsFound: boolean
    renderDistance: number | null
    shadersEnabled: boolean
    shaderPack: string | null
    shaderPackAvailable: boolean
    /** Own settings saved before the last switch to a preset ('custom' restores them). */
    hasBackup: boolean
}

export async function galaxyGraphicsGet(): Promise<GalaxyGraphicsState> {
    return await invoke<GalaxyGraphicsState>('plugin:galaxy|galaxy_graphics_get')
}

/** Refused while the game runs. */
export async function galaxyGraphicsApply(
    preset: GalaxyGraphicsPreset | 'custom',
): Promise<GalaxyGraphicsState> {
    return await invoke<GalaxyGraphicsState>('plugin:galaxy|galaxy_graphics_apply', { preset })
}

/** Returns a Chinese validation message, or null when the name is acceptable. */
export function validateGalaxyPlayerName(name: string): string | null {
    const trimmed = name.trim()
    if (!trimmed) return '请输入游戏名'
    if (trimmed.length < 3 || trimmed.length > 16) return '游戏名长度需为 3–16 个字符'
    if (!GALAXY_PLAYER_NAME_PATTERN.test(trimmed)) return '游戏名只能包含英文字母、数字和下划线'
    return null
}

/** Normalises any invoke rejection into a readable string. */
export function galaxyErrorText(error: unknown): string {
    if (typeof error === 'string') return error
    if (error instanceof Error) return error.message
    if (error && typeof error === 'object' && 'message' in error) {
        const message = (error as { message?: unknown }).message
        if (typeof message === 'string') return message
    }
    try {
        return JSON.stringify(error)
    } catch {
        return String(error)
    }
}
