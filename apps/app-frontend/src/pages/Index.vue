<script setup lang="ts">
/**
 * Galaxy宝可梦 home page — the "server lobby" (GALAXY_LAYOUT_V2.md): a hero
 * banner with the live server status, announcements next to a server-info and
 * quick-actions column, and a sticky play dock at the bottom. Replaces
 * Axolotl's dashboard / minimal home, Minecraft news, play insights and daily
 * challenge.
 */
import {
    CalendarIcon,
    CheckCircleIcon,
    CheckIcon,
    ChevronRightIcon,
    CopyIcon,
    DownloadIcon,
    FolderOpenIcon,
    InfoIcon,
    MonitorIcon,
    NoSignalIcon,
    PlayIcon,
    RefreshCwIcon,
    ServerIcon,
    SettingsIcon,
    SpinnerIcon,
    StopCircleIcon,
    TerminalSquareIcon,
    TriangleAlertIcon,
    UserIcon,
    UserPlusIcon,
    WrenchIcon,
} from '@modrinth/assets'
import {
    Admonition,
    Avatar,
    Button,
    ConfirmModal,
    defineMessages,
    injectNotificationManager,
    NewModal,
    useVIntl,
} from '@modrinth/ui'
import { autoToHTML } from '@sfirew/minecraft-motd-parser'
import { useQueryClient } from '@tanstack/vue-query'
import { computed, inject, onUnmounted, type Ref, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import galaxyLogo from '@/assets/galaxy-logo.svg?url'
import GalaxyEventsList from '@/components/ui/GalaxyEventsList.vue'
import GalaxyGraphicsModal from '@/components/ui/GalaxyGraphicsModal.vue'
import GalaxyMinecraftText from '@/components/ui/GalaxyMinecraftText.vue'
import GalaxyOnboardingCard from '@/components/ui/GalaxyOnboardingCard.vue'
import GalaxyPlayerNameModal from '@/components/ui/GalaxyPlayerNameModal.vue'
import GalaxyPlayerStats from '@/components/ui/GalaxyPlayerStats.vue'
import {
    fetchCachedServerStatus,
    getServerStatusQueryKey,
} from '@/composables/instances/use-server-status-query'
import { useMinecraftLaunchError } from '@/composables/useMinecraftLaunchError'
import { formatGalaxyDate } from '@/galaxy/announcements'
import { markGalaxyLaunched } from '@/galaxy/onboarding'
import { getDefaultHeadForName } from '@/galaxy/player-head'
import { useGalaxyAnnouncements } from '@/galaxy/use-announcements'
import { useGalaxyPlayer, useGalaxyServerSummary } from '@/galaxy/use-server-data'
import { get_default_user, users } from '@/helpers/auth'
import { process_listener } from '@/helpers/events'
import { GALAXY_INSTANCE_NAME, galaxyErrorText } from '@/helpers/galaxy'
import { install_existing_instance } from '@/helpers/install'
import { get as getInstance, kill, run } from '@/helpers/instance'
import { get_by_instance_id } from '@/helpers/process'
import type { GameInstance } from '@/helpers/types'
import { showInstanceInFolder } from '@/helpers/utils.js'
import type { ServerStatus } from '@/helpers/worlds'
import { useBreadcrumbs } from '@/store/breadcrumbs'
import { handleSevereError } from '@/store/error.js'
import { useGalaxyStore } from '@/store/galaxy'

type AccountsCardHandle = {
    refreshValues?: () => Promise<void>
    login?: () => void
    selectedName?: string | null
    avatarUrl?: string | null
}

const { handleError, addNotification } = injectNotificationManager()
const { formatMessage } = useVIntl()
const route = useRoute()
const router = useRouter()
const breadcrumbs = useBreadcrumbs()
const queryClient = useQueryClient()
const galaxy = useGalaxyStore()
const handleMinecraftLaunchError = useMinecraftLaunchError()
const accountsCard = inject<Ref<AccountsCardHandle | null> | null>('accountsCard', null)
const openPlayerMenu = inject<(() => void) | null>('galaxyOpenPlayerMenu', null)

const messages = defineMessages({
    home: { id: 'app.home.breadcrumb', defaultMessage: 'Home' },
})

breadcrumbs.setRootContext({ name: formatMessage(messages.home), link: route.path })

const TEXT = {
    loading: '正在查找游戏目录…',
    errorTitle: '无法启动 Galaxy宝可梦',
    notFound:
        '没有找到游戏目录（.minecraft）。请确认启动器放在安装目录的 launcher 文件夹里，或重新安装客户端。',
    retry: '重新检测',
    play: '开始游戏',
    starting: '启动中… {seconds} 秒',
    stopping: '正在停止…',
    stop: '停止游戏',
    online: '在线',
    offline: '离线',
    checking: '正在获取服务器状态…',
    players: '{online}/{max} 人在线',
    ping: '{ping} 毫秒',
    refreshStatus: '刷新状态',
    announcements: '公告',
    pinned: '置顶',
    server: '服务器',
    address: '地址',
    copy: '复制',
    copied: '已复制',
    copyFailed: '复制地址失败',
    version: '版本',
    onlinePlayers: '在线玩家',
    noOnlinePlayers: '暂时没有玩家在线',
    quickActions: '快捷操作',
    instanceSettings: '游戏设置',
    instanceSettingsHint: '内存、Java、窗口大小',
    openFolder: '打开游戏文件夹',
    viewLogs: '查看日志',
    repair: '修复游戏文件',
    repairTitle: '修复游戏文件？',
    repairDescription:
        '启动器会重新检查并下载缺失或损坏的 Minecraft 与加载器文件，再逐个校验整合包里的文件：缺失或被改动的会重新下载，原来的文件放进备份。修复期间请不要启动游戏。',
    repairProceed: '开始修复',
    repairStarted: '已开始修复游戏文件',
    repairStartedText: 'Minecraft 文件的进度显示在顶部的下载按钮里，整合包的进度显示在下方。',
    playingAs: '以 {name} 进入服务器',
    switchName: '切换',
    noName: '还没有设置游戏名',
    setName: '设置游戏名',
    syncChecking: '正在检查整合包更新…',
    syncHashingFirst: '首次检查整合包文件 {done}/{total}…',
    syncHashingRepair: '正在校验整合包文件 {done}/{total}…',
    syncDownloading: '正在更新整合包 {done}/{total} · {doneMb}/{totalMb} MB',
    syncApplying: '正在安装更新…',
    syncUpToDate: '整合包已是最新 · {version}',
    syncUpdateWhilePlaying: '整合包有更新 {version}，关闭游戏后自动安装',
    syncUpdateAvailable: '整合包有更新 {version}',
    syncOffline: '暂时连不上更新服务器，将使用本地整合包',
    syncFailed: '整合包更新失败',
    syncRetry: '重试',
    syncUpdateNow: '立即更新',
    syncUpdating: '更新中…',
    syncPlayBlocked: '整合包不完整，请联网更新后再开始游戏',
    syncUpdatedTitle: '整合包已更新到 {version}',
    syncUpdatedText: '更新了 {files} 个文件。',
    syncUpdatedBackup: '被替换的旧文件保存在 .galaxy-sync\\backups 里。',
    syncDetailsTitle: '整合包版本',
    syncDetailsCurrent: '当前版本',
    syncDetailsReleased: '发布时间',
    syncDetailsChangelog: '更新说明',
    syncDetailsHistory: '历史版本',
    syncDetailsNoChangelog: '这个版本没有填写更新说明',
    syncDetailsClose: '关闭',
    preparing: '正在准备游戏…',
    unavailable: '暂时无法启动，请查看上方的提示',
    mustRead: '必读',
    viewAll: '查看全部 ({count})',
    noAnnouncements: '暂时没有公告',
    maintenance: '维护中',
    maintenanceTitle: '服务器维护中',
    maintenanceDefault: '请稍后再来，维护结束后就能进服。',
    tps: '流畅度',
    tpsValue: 'TPS {tps}',
    mainServerUnknown: '主服数据暂时获取不到，显示的是入口服务器的状态。',
    myData: '我的数据',
    events: '活动与赛季',
    graphics: '画质档位',
    graphicsHint: '低配流畅 · 均衡 · 高画质（光影）',
}

function fill(template: string, values: Record<string, string | number>) {
    return template.replace(/\{(\w+)\}/g, (_, key) => String(values[key] ?? ''))
}

// ---- instance + server ------------------------------------------------------
const instance = ref<GameInstance | null>(null)
const instanceName = computed(() => instance.value?.name || GALAXY_INSTANCE_NAME)
const serverAddress = computed(() => galaxy.serverAddress)
const { announcements, now: announcementsNow } = useGalaxyAnnouncements()
const HOME_ANNOUNCEMENT_LIMIT = 3
const homeAnnouncements = computed(() => announcements.value.slice(0, HOME_ANNOUNCEMENT_LIMIT))
const instancePath = computed(() =>
    galaxy.instanceId ? `/instance/${encodeURIComponent(galaxy.instanceId)}` : null,
)
const loaderLabel = computed(() => {
    const loader = instance.value?.loader
    if (!loader || loader === 'vanilla') return null
    const name = loader.charAt(0).toUpperCase() + loader.slice(1)
    return instance.value?.loader_version ? `${name} ${instance.value.loader_version}` : name
})

const status = ref<ServerStatus | null>(null)
const statusLoading = ref(false)
const statusFailed = ref(false)
const renderedMotd = computed(() => {
    const description = status.value?.description
    if (!description) return ''
    try {
        return autoToHTML(description as Parameters<typeof autoToHTML>[0])
    } catch {
        return ''
    }
})
const samplePlayers = computed(() =>
    (status.value?.players?.sample ?? [])
        .map((player) => player.name)
        .filter((name) => !!name && /^[A-Za-z0-9_]{1,16}$/.test(name))
        .slice(0, 12),
)
const serverVersion = computed(() => status.value?.version?.name || instance.value?.game_version)

// ---- live main-server data (GalaxyLauncherApi, GALAXY_LAUNCHER_API.md) --------
// The Velocity ping above only describes the proxy (ping-passthrough is off),
// so online count, players and maintenance come from the data API when it answers.
const {
    summary: liveSummary,
    state: summaryState,
    refresh: refreshSummary,
} = useGalaxyServerSummary()
const liveServer = computed(() => liveSummary.value?.server ?? null)
const maintenance = computed(() =>
    liveServer.value?.maintenance.active ? liveServer.value.maintenance : null,
)
const liveEvents = computed(() => liveSummary.value?.events ?? [])
const onlineCount = computed(() => liveServer.value?.online ?? status.value?.players?.online ?? 0)
const maxPlayers = computed(() => liveServer.value?.max ?? status.value?.players?.max ?? 0)
const hasPlayerCount = computed(() => !!liveServer.value || !!status.value)
const onlineNames = computed(() =>
    liveServer.value
        ? liveServer.value.players.map((player) => player.name).slice(0, 24)
        : samplePlayers.value,
)
const mainServerUnknown = computed(() => summaryState.value === 'error' && !!status.value)
const tpsText = computed(() => {
    const tps = liveServer.value?.tps
    return tps === null || tps === undefined ? null : fill(TEXT.tpsValue, { tps: tps.toFixed(1) })
})

const playerHeads = ref<Record<string, string>>({})
watch(
    onlineNames,
    async (names) => {
        for (const name of names) {
            if (playerHeads.value[name]) continue
            const head = await getDefaultHeadForName(name)
            if (head) playerHeads.value = { ...playerHeads.value, [name]: head }
        }
    },
    { immediate: true },
)

async function loadStatus(force = false) {
    const address = serverAddress.value
    if (!address) return
    statusLoading.value = true
    statusFailed.value = false
    try {
        if (force) queryClient.removeQueries({ queryKey: getServerStatusQueryKey(address) })
        status.value = await fetchCachedServerStatus(queryClient, address)
    } catch (error) {
        console.warn(`[galaxy] failed to ping ${address}`, error)
        status.value = null
        statusFailed.value = true
    } finally {
        statusLoading.value = false
    }
}

const addressCopied = ref(false)
let addressCopiedTimer: ReturnType<typeof setTimeout> | undefined

async function copyAddress() {
    try {
        await navigator.clipboard.writeText(serverAddress.value)
    } catch (error) {
        addNotification({ type: 'error', title: TEXT.copyFailed, text: galaxyErrorText(error) })
        return
    }
    addressCopied.value = true
    if (addressCopiedTimer) clearTimeout(addressCopiedTimer)
    addressCopiedTimer = setTimeout(() => {
        addressCopied.value = false
    }, 2000)
}

// ---- play state -------------------------------------------------------------
const playing = ref(false)
const launching = ref(false)
const stopping = ref(false)
const launchElapsedSeconds = ref(0)
let launchTimer: ReturnType<typeof setInterval> | undefined

async function refreshInstance(instanceId: string | null) {
    if (!instanceId) {
        instance.value = null
        playing.value = false
        return
    }
    instance.value = await getInstance(instanceId).catch((error) => {
        console.warn('[galaxy] failed to read the fixed instance', error)
        return null
    })
    const processes = await get_by_instance_id(instanceId).catch(() => [])
    playing.value = Array.isArray(processes) && processes.length > 0
}

watch(
    () => galaxy.instanceId,
    (instanceId) => void refreshInstance(instanceId),
    { immediate: true },
)
watch(serverAddress, () => void loadStatus(), { immediate: true })

// ---- player name ------------------------------------------------------------
const currentName = computed(() => accountsCard?.value?.selectedName ?? null)
const currentAvatar = computed(() => accountsCard?.value?.avatarUrl ?? null)
const {
    player: livePlayer,
    known: playerKnown,
    refresh: refreshPlayer,
} = useGalaxyPlayer(currentName)
const showPlayerStats = computed(() => livePlayer.value?.known === true)

let afterGameTimer: ReturnType<typeof setTimeout> | undefined

const unlistenProcesses = await process_listener((e: { event: string; instance_id: string }) => {
    if (e.instance_id !== galaxy.instanceId) return
    if (e.event === 'launched') {
        playing.value = true
        markGalaxyLaunched()
    }
    if (e.event === 'finished') {
        playing.value = false
        // The server saves the player's data a moment after they leave.
        if (afterGameTimer) clearTimeout(afterGameTimer)
        afterGameTimer = setTimeout(() => {
            void refreshPlayer(true)
            void refreshSummary()
        }, 5000)
    }
})

onUnmounted(() => {
    unlistenProcesses()
    if (launchTimer) clearInterval(launchTimer)
    if (addressCopiedTimer) clearTimeout(addressCopiedTimer)
    if (afterGameTimer) clearTimeout(afterGameTimer)
})

const nameModal = ref<InstanceType<typeof GalaxyPlayerNameModal> | null>(null)

async function hasSelectedOfflineAccount() {
    const [selected, list] = await Promise.all([
        get_default_user(true).catch(() => null),
        users(true, true).catch(() => []),
    ])
    if (!selected || !Array.isArray(list)) return false
    return list.some(
        (account: { account_id?: string; account_type?: string }) =>
            account.account_id === selected && account.account_type === 'offline',
    )
}

function openNameModal() {
    void nameModal.value?.show()
}

function switchName() {
    if (openPlayerMenu) openPlayerMenu()
    else accountsCard?.value?.login?.()
}

/** The dialog already called galaxySetPlayer; refresh the header chip, then launch. */
async function onNameSaved() {
    await accountsCard?.value?.refreshValues?.()
    await launch()
}

// ---- launch / stop ----------------------------------------------------------
async function play() {
    if (!galaxy.ready || launching.value || playing.value) return
    if (galaxy.syncBusy || galaxy.syncBlocksPlay) return
    if (!(await hasSelectedOfflineAccount())) {
        openNameModal()
        return
    }
    await launch()
}

/**
 * Quick modpack check before every launch (cheap: the hash cache answers) that
 * installs a pending update first. Offline is fine: the local pack is used.
 */
async function syncBeforeLaunch(): Promise<boolean> {
    const result = await galaxy.sync()
    if (!result) return false
    if (result.status === 'upToDate' || result.status === 'offline') return true
    addNotification({
        type: 'error',
        title: TEXT.syncPlayBlocked,
        text: result.message ?? '',
    })
    return false
}

async function launch() {
    const instanceId = galaxy.instanceId
    if (!instanceId || launching.value) return
    galaxy.launchInProgress = true
    try {
        if (!(await syncBeforeLaunch())) return
        launching.value = true
        launchElapsedSeconds.value = 0
        launchTimer = setInterval(() => {
            launchElapsedSeconds.value += 1
        }, 1000)
        await run(instanceId, serverAddress.value)
        playing.value = true
    } catch (error) {
        const handled = await handleMinecraftLaunchError(error, {
            instance_id: instanceId,
            instance_name: instanceName.value,
        })
        if (!handled) handleSevereError(error, { instanceId })
    } finally {
        if (launchTimer) clearInterval(launchTimer)
        launchTimer = undefined
        launching.value = false
        galaxy.launchInProgress = false
    }
}

async function stop() {
    const instanceId = galaxy.instanceId
    if (!instanceId) return
    stopping.value = true
    await kill(instanceId).catch(handleError)
    stopping.value = false
    playing.value = false
}

// ---- quick actions ----------------------------------------------------------
const repairModal = ref<InstanceType<typeof ConfirmModal> | null>(null)
const graphicsModal = ref<InstanceType<typeof GalaxyGraphicsModal> | null>(null)

async function refreshServerInfo() {
    await Promise.all([loadStatus(true), refreshSummary(), refreshPlayer(true)])
}

function openInstanceSettings() {
    if (instancePath.value) void router.push({ path: instancePath.value, query: { settings: '1' } })
}

function openLogs() {
    if (instancePath.value) void router.push(`${instancePath.value}/logs`)
}

async function openFolder() {
    if (galaxy.instanceId) await showInstanceInFolder(galaxy.instanceId)
}

async function repairGameFiles() {
    const instanceId = galaxy.instanceId
    if (!instanceId) return
    addNotification({ type: 'info', title: TEXT.repairStarted, text: TEXT.repairStartedText })
    await install_existing_instance(instanceId, false).catch(handleError)
    // Then re-hash every modpack file (ignores the hash cache) and reinstall what differs.
    if (galaxy.syncBusy) await galaxy.sync()
    await galaxy.sync({ force: true })
}

async function retryBootstrap() {
    await galaxy.bootstrap(true)
}

const showError = computed(
    () => !galaxy.loading && (!!galaxy.error || (!!galaxy.payload && !galaxy.gameFound)),
)
const errorText = computed(() => galaxy.error || TEXT.notFound)
const dockNotice = computed(() => {
    if (playing.value || launching.value) return null
    if (galaxy.loading && !galaxy.payload) return TEXT.preparing
    if (!galaxy.ready) return TEXT.unavailable
    return null
})

// ---- modpack sync (built-in updater, GALAXY_SYNC.md) --------------------------
type SyncTone = 'neutral' | 'ok' | 'warn' | 'danger'
interface SyncView {
    tone: SyncTone
    icon: 'spinner' | 'ok' | 'offline' | 'update' | 'alert'
    text: string
    /** 0..1 for the progress bar, null for none. */
    progress: number | null
    action: 'details' | 'retry' | 'apply' | null
    title: string | null
}

function megabytes(bytes: number) {
    const value = bytes / 1024 / 1024
    return value >= 100 ? value.toFixed(0) : value.toFixed(1)
}

function ratio(done: number, total: number) {
    return total > 0 ? Math.min(1, Math.max(0, done / total)) : null
}

/** "2026-10-31T17:47:32.741885600Z" → local "2026-11-01 01:47". */
function formatReleaseTime(value: string | null | undefined) {
    if (!value) return ''
    const date = new Date(value.replace(/(\.\d{3})\d+/, '$1'))
    if (Number.isNaN(date.getTime())) return value
    const pad = (n: number) => String(n).padStart(2, '0')
    return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`
}

const syncView = computed<SyncView | null>(() => {
    if (!galaxy.ready) return null
    const progress = galaxy.syncProgress
    const busy = (
        text: string,
        value: number | null = null,
        title: string | null = null,
    ): SyncView => ({
        tone: 'neutral',
        icon: 'spinner',
        text,
        progress: value,
        action: null,
        title,
    })
    if (galaxy.syncRunning) {
        if (progress?.phase === 'hashing' && progress.totalFiles > 0)
            return busy(
                fill(galaxy.syncForced ? TEXT.syncHashingRepair : TEXT.syncHashingFirst, {
                    done: progress.doneFiles,
                    total: progress.totalFiles,
                }),
                ratio(progress.doneBytes, progress.totalBytes),
            )
        if (progress?.phase === 'downloading')
            return busy(
                fill(TEXT.syncDownloading, {
                    done: progress.doneFiles,
                    total: progress.totalFiles,
                    doneMb: megabytes(progress.doneBytes),
                    totalMb: megabytes(progress.totalBytes),
                }),
                ratio(progress.doneBytes, progress.totalBytes) ?? 0,
                progress.currentFile,
            )
        if (progress?.phase === 'applying') return busy(TEXT.syncApplying)
        return busy(TEXT.syncChecking)
    }
    const result = galaxy.syncResult
    if (!result) return busy(TEXT.syncChecking)
    const version = result.remoteVersion || result.localVersion || ''
    switch (result.status) {
        case 'upToDate':
            return {
                tone: 'ok',
                icon: 'ok',
                text: version
                    ? fill(TEXT.syncUpToDate, { version })
                    : TEXT.syncUpToDate.split(' · ')[0],
                progress: null,
                action: 'details',
                title: null,
            }
        case 'updateAvailable':
            return {
                tone: 'warn',
                icon: 'update',
                text: fill(playing.value ? TEXT.syncUpdateWhilePlaying : TEXT.syncUpdateAvailable, {
                    version,
                }),
                progress: null,
                action: playing.value ? null : 'apply',
                title: null,
            }
        case 'offline':
            return {
                tone: 'warn',
                icon: 'offline',
                text: TEXT.syncOffline,
                progress: null,
                action: 'retry',
                title: result.message,
            }
        default:
            return {
                tone: 'danger',
                icon: 'alert',
                text: result.message || TEXT.syncFailed,
                progress: null,
                action: 'retry',
                title: result.message,
            }
    }
})

async function retrySync() {
    await galaxy.sync()
}

const syncDetailsModal = ref<InstanceType<typeof NewModal> | null>(null)
const syncDetails = computed(() => {
    const result = galaxy.syncResult
    return {
        version: result?.remoteVersion || result?.localVersion || '—',
        releasedAt: formatReleaseTime(result?.releasedAt),
        changelog: result?.changelog?.trim() || '',
        releaseId: result?.releaseId ?? null,
        history: (result?.history ?? []).slice(0, 10),
    }
})

function openSyncDetails() {
    syncDetailsModal.value?.show()
}

// The store sets this once per installed update (it may happen before this page mounts).
watch(
    () => galaxy.appliedNotice,
    (result) => {
        if (!result) return
        galaxy.appliedNotice = null
        addNotification({
            type: 'success',
            title: fill(TEXT.syncUpdatedTitle, { version: result.remoteVersion ?? '' }),
            text:
                fill(TEXT.syncUpdatedText, { files: result.filesInstalled }) +
                (result.filesBackedUp > 0 ? TEXT.syncUpdatedBackup : ''),
        })
    },
    { immediate: true },
)
</script>

<template>
    <div class="galaxy-lobby mx-auto flex min-h-full w-full max-w-[76rem] flex-col gap-5 px-6 pt-6">
        <!-- Hero banner -->
        <section class="galaxy-hero relative overflow-hidden rounded-3xl">
            <div class="galaxy-hero-pattern" aria-hidden="true" />
            <div class="relative flex items-center gap-6 px-8 py-7">
                <div class="galaxy-hero-icon shrink-0">
                    <img
                        :src="status?.favicon || galaxyLogo"
                        :alt="instanceName"
                        :class="{ pixelated: !!status?.favicon }"
                    />
                </div>
                <div class="flex min-w-0 flex-1 flex-col gap-3">
                    <h1 class="m-0 truncate text-4xl font-extrabold leading-tight text-white">
                        {{ instanceName }}
                    </h1>
                    <div v-if="renderedMotd" class="galaxy-hero-motd self-start">
                        <p
                            class="motd-renderer m-0 font-minecraft line-clamp-2 leading-5"
                            v-html="renderedMotd"
                        />
                    </div>
                    <div class="flex flex-wrap items-center gap-2">
                        <span v-if="maintenance" class="galaxy-hero-chip is-maintenance">
                            <WrenchIcon aria-hidden="true" />
                            {{ TEXT.maintenance }}
                        </span>
                        <span
                            v-if="statusLoading && !status && !liveServer"
                            class="galaxy-hero-chip"
                        >
                            <SpinnerIcon class="animate-spin" aria-hidden="true" />
                            {{ TEXT.checking }}
                        </span>
                        <template v-else-if="status || liveServer">
                            <span v-if="!maintenance" class="galaxy-hero-chip">
                                <span class="galaxy-online-dot" aria-hidden="true" />
                                {{ TEXT.online }}
                                {{ onlineCount }}/{{ maxPlayers }}
                            </span>
                            <span v-if="status?.ping" class="galaxy-hero-chip">
                                {{ fill(TEXT.ping, { ping: status.ping }) }}
                            </span>
                        </template>
                        <span v-else-if="statusFailed" class="galaxy-hero-chip">
                            <NoSignalIcon aria-hidden="true" />
                            {{ TEXT.offline }}
                        </span>
                        <span v-if="instance?.game_version" class="galaxy-hero-chip">
                            Minecraft {{ instance.game_version }}
                        </span>
                        <span v-if="loaderLabel" class="galaxy-hero-chip">{{ loaderLabel }}</span>
                    </div>
                </div>
            </div>
        </section>

        <Admonition v-if="galaxy.loading && !galaxy.payload" type="info" :header="TEXT.loading" />
        <Admonition v-else-if="showError" type="critical" :header="TEXT.errorTitle">
            <p class="m-0">{{ errorText }}</p>
            <Button class="mt-3" :disabled="galaxy.loading" @click="retryBootstrap()"
                ><RefreshCwIcon :class="{ 'animate-spin': galaxy.loading }" />
                {{ TEXT.retry }}
            </Button>
        </Admonition>
        <Admonition v-if="maintenance" type="warning" :header="TEXT.maintenanceTitle">
            <p class="m-0">{{ maintenance.message || TEXT.maintenanceDefault }}</p>
        </Admonition>

        <GalaxyOnboardingCard
            :has-name="!!currentName"
            :known="playerKnown"
            :play-disabled="!galaxy.ready || galaxy.syncBusy || launching || playing"
            @set-name="openNameModal()"
            @play="play()"
        />

        <div class="galaxy-lobby-grid">
            <!-- Announcements -->
            <section class="flex min-w-0 flex-col gap-3">
                <div class="galaxy-section-title">
                    <InfoIcon aria-hidden="true" />
                    <h2>{{ TEXT.announcements }}</h2>
                    <router-link
                        v-if="announcements.length"
                        to="/announcements"
                        class="galaxy-view-all"
                        >{{ fill(TEXT.viewAll, { count: announcements.length }) }}
                        <ChevronRightIcon aria-hidden="true"
                    /></router-link>
                </div>
                <router-link
                    v-for="item in homeAnnouncements"
                    :key="item.id"
                    :to="{ path: '/announcements', hash: `#announcement-${item.id}` }"
                    class="galaxy-card galaxy-announcement flex flex-col gap-1.5 px-4 py-3"
                >
                    <div class="flex min-w-0 items-center gap-1.5">
                        <span class="galaxy-badge">{{ item.category }}</span>
                        <span v-if="item.pinned" class="galaxy-badge is-pinned">{{
                            TEXT.pinned
                        }}</span>
                        <span v-if="item.mustRead" class="galaxy-badge is-must-read">{{
                            TEXT.mustRead
                        }}</span>
                        <time
                            class="ml-auto shrink-0 text-xs text-[var(--color-text-tertiary)]"
                            :datetime="item.createdAt"
                            >{{ formatGalaxyDate(item.createdAt, announcementsNow) }}</time
                        >
                    </div>
                    <h3 class="m-0 truncate text-base font-bold text-[var(--color-text-primary)]">
                        <GalaxyMinecraftText :text="item.title" />
                    </h3>
                    <p
                        class="galaxy-announcement-preview m-0 text-sm leading-6 text-[var(--color-text-default)]"
                    >
                        <template v-for="(line, index) in item.content" :key="index"
                            ><GalaxyMinecraftText :text="line" />{{ ' ' }}</template
                        >
                    </p>
                </router-link>
                <div
                    v-if="!announcements.length"
                    class="galaxy-card p-4 text-sm text-[var(--color-text-tertiary)]"
                >
                    {{ TEXT.noAnnouncements }}
                </div>
            </section>

            <!-- My data + server info + events + quick actions -->
            <aside class="flex min-w-0 flex-col gap-3">
                <template v-if="showPlayerStats && livePlayer">
                    <div class="galaxy-section-title">
                        <UserIcon aria-hidden="true" />
                        <h2>{{ TEXT.myData }}</h2>
                    </div>
                    <div class="galaxy-card p-4">
                        <GalaxyPlayerStats :player="livePlayer" :now="announcementsNow" />
                    </div>
                </template>

                <div class="galaxy-section-title" :class="{ 'mt-2': showPlayerStats }">
                    <ServerIcon aria-hidden="true" />
                    <h2>{{ TEXT.server }}</h2>
                </div>
                <div class="galaxy-card flex flex-col gap-3 p-4">
                    <div class="galaxy-info-row">
                        <span class="galaxy-info-label">{{ TEXT.address }}</span>
                        <div class="flex min-w-0 items-center gap-1.5">
                            <code class="galaxy-address truncate">{{ serverAddress }}</code>
                            <button
                                v-tooltip="addressCopied ? TEXT.copied : TEXT.copy"
                                type="button"
                                class="galaxy-icon-button"
                                :aria-label="TEXT.copy"
                                @click="copyAddress()"
                            >
                                <CheckIcon v-if="addressCopied" class="text-brand" />
                                <CopyIcon v-else />
                            </button>
                        </div>
                    </div>
                    <div v-if="serverVersion" class="galaxy-info-row">
                        <span class="galaxy-info-label">{{ TEXT.version }}</span>
                        <span
                            class="truncate text-sm font-medium text-[var(--color-text-primary)]"
                            >{{ serverVersion }}</span
                        >
                    </div>
                    <div class="galaxy-info-row">
                        <span class="galaxy-info-label">{{ TEXT.onlinePlayers }}</span>
                        <span
                            v-if="hasPlayerCount"
                            class="text-sm font-medium text-[var(--color-text-primary)]"
                            >{{
                                fill(TEXT.players, { online: onlineCount, max: maxPlayers })
                            }}</span
                        >
                        <span
                            v-else-if="statusFailed"
                            class="text-sm font-medium text-[var(--color-text-tertiary)]"
                            >{{ TEXT.offline }}</span
                        >
                        <span v-else class="text-sm text-[var(--color-text-tertiary)]">—</span>
                    </div>
                    <div v-if="tpsText" class="galaxy-info-row">
                        <span class="galaxy-info-label">{{ TEXT.tps }}</span>
                        <span class="text-sm font-medium text-[var(--color-text-primary)]">{{
                            tpsText
                        }}</span>
                    </div>
                    <ul v-if="onlineNames.length" class="galaxy-online-heads">
                        <li v-for="name in onlineNames" :key="name" v-tooltip="name">
                            <img
                                v-if="playerHeads[name]"
                                :src="playerHeads[name]"
                                :alt="name"
                                class="pixelated"
                            />
                            <span v-else class="galaxy-online-head-fallback" aria-hidden="true">{{
                                name.charAt(0).toUpperCase()
                            }}</span>
                            <span class="galaxy-online-name">{{ name }}</span>
                        </li>
                    </ul>
                    <p
                        v-else-if="hasPlayerCount && onlineCount === 0"
                        class="m-0 text-xs text-[var(--color-text-tertiary)]"
                    >
                        {{ TEXT.noOnlinePlayers }}
                    </p>
                    <p
                        v-if="mainServerUnknown"
                        class="m-0 text-xs text-[var(--color-text-tertiary)]"
                    >
                        {{ TEXT.mainServerUnknown }}
                    </p>
                    <Button class="w-full" :disabled="statusLoading" @click="refreshServerInfo()"
                        ><RefreshCwIcon :class="{ 'animate-spin': statusLoading }" />
                        {{ TEXT.refreshStatus }}
                    </Button>
                </div>

                <template v-if="liveEvents.length">
                    <div class="galaxy-section-title mt-2">
                        <CalendarIcon aria-hidden="true" />
                        <h2>{{ TEXT.events }}</h2>
                    </div>
                    <div class="galaxy-card p-3">
                        <GalaxyEventsList :events="liveEvents" :now="announcementsNow" />
                    </div>
                </template>

                <div class="galaxy-section-title mt-2">
                    <WrenchIcon aria-hidden="true" />
                    <h2>{{ TEXT.quickActions }}</h2>
                </div>
                <div class="galaxy-card flex flex-col p-1.5">
                    <button
                        type="button"
                        class="galaxy-action"
                        :disabled="!galaxy.instanceId"
                        @click="openInstanceSettings()"
                    >
                        <SettingsIcon aria-hidden="true" />
                        <span class="flex min-w-0 flex-col items-start">
                            <span>{{ TEXT.instanceSettings }}</span>
                            <span class="text-xs font-normal text-[var(--color-text-tertiary)]">{{
                                TEXT.instanceSettingsHint
                            }}</span>
                        </span>
                    </button>
                    <button
                        type="button"
                        class="galaxy-action"
                        :disabled="!galaxy.ready"
                        @click="graphicsModal?.show()"
                    >
                        <MonitorIcon aria-hidden="true" />
                        <span class="flex min-w-0 flex-col items-start">
                            <span>{{ TEXT.graphics }}</span>
                            <span class="text-xs font-normal text-[var(--color-text-tertiary)]">{{
                                TEXT.graphicsHint
                            }}</span>
                        </span>
                    </button>
                    <button
                        type="button"
                        class="galaxy-action"
                        :disabled="!galaxy.instanceId"
                        @click="openFolder()"
                    >
                        <FolderOpenIcon aria-hidden="true" />
                        <span>{{ TEXT.openFolder }}</span>
                    </button>
                    <button
                        type="button"
                        class="galaxy-action"
                        :disabled="!galaxy.instanceId"
                        @click="openLogs()"
                    >
                        <TerminalSquareIcon aria-hidden="true" />
                        <span>{{ TEXT.viewLogs }}</span>
                    </button>
                    <button
                        type="button"
                        class="galaxy-action"
                        :disabled="!galaxy.ready || playing || launching || galaxy.syncBusy"
                        @click="repairModal?.show()"
                    >
                        <WrenchIcon aria-hidden="true" />
                        <span>{{ TEXT.repair }}</span>
                    </button>
                </div>
            </aside>
        </div>

        <!-- Play dock -->
        <div class="galaxy-dock-wrap">
            <div class="galaxy-dock">
                <div class="flex min-w-0 items-center gap-3">
                    <template v-if="currentName">
                        <Avatar
                            :src="currentAvatar || galaxyLogo"
                            size="40px"
                            pixelated
                            :unframed-natural-width="72"
                            class="shrink-0"
                        />
                        <div class="flex min-w-0 flex-col">
                            <span
                                class="truncate text-sm font-semibold text-[var(--color-text-primary)]"
                                >{{ fill(TEXT.playingAs, { name: currentName }) }}</span
                            >
                            <button type="button" class="galaxy-link" @click="switchName()">
                                {{ TEXT.switchName }}
                            </button>
                        </div>
                    </template>
                    <template v-else>
                        <span class="galaxy-dock-placeholder shrink-0">
                            <UserPlusIcon aria-hidden="true" />
                        </span>
                        <div class="flex min-w-0 flex-col">
                            <span
                                class="truncate text-sm font-semibold text-[var(--color-text-primary)]"
                                >{{ TEXT.noName }}</span
                            >
                            <button type="button" class="galaxy-link" @click="openNameModal()">
                                {{ TEXT.setName }}
                            </button>
                        </div>
                    </template>
                </div>
                <div class="flex min-w-0 flex-1 justify-center">
                    <span
                        v-if="dockNotice"
                        class="truncate text-sm text-[var(--color-text-tertiary)]"
                        >{{ dockNotice }}</span
                    >
                    <div
                        v-else-if="syncView"
                        class="galaxy-sync"
                        :class="`is-${syncView.tone}`"
                        :title="syncView.title ?? undefined"
                        role="status"
                        aria-live="polite"
                    >
                        <div class="galaxy-sync-line">
                            <SpinnerIcon
                                v-if="syncView.icon === 'spinner'"
                                class="animate-spin"
                                aria-hidden="true"
                            />
                            <CheckCircleIcon
                                v-else-if="syncView.icon === 'ok'"
                                aria-hidden="true"
                            />
                            <NoSignalIcon
                                v-else-if="syncView.icon === 'offline'"
                                aria-hidden="true"
                            />
                            <DownloadIcon
                                v-else-if="syncView.icon === 'update'"
                                aria-hidden="true"
                            />
                            <TriangleAlertIcon v-else aria-hidden="true" />
                            <button
                                v-if="syncView.action === 'details'"
                                type="button"
                                class="galaxy-sync-text is-clickable"
                                @click="openSyncDetails()"
                            >
                                {{ syncView.text }}
                            </button>
                            <span v-else class="galaxy-sync-text">{{ syncView.text }}</span>
                            <button
                                v-if="syncView.action === 'retry'"
                                type="button"
                                class="galaxy-link galaxy-sync-action"
                                @click="retrySync()"
                            >
                                {{ TEXT.syncRetry }}
                            </button>
                            <button
                                v-else-if="syncView.action === 'apply'"
                                type="button"
                                class="galaxy-link galaxy-sync-action"
                                @click="retrySync()"
                            >
                                {{ TEXT.syncUpdateNow }}
                            </button>
                        </div>
                        <div
                            v-if="syncView.progress !== null"
                            class="galaxy-sync-bar"
                            role="progressbar"
                            aria-valuemin="0"
                            aria-valuemax="100"
                            :aria-valuenow="Math.round(syncView.progress * 100)"
                        >
                            <span :style="{ width: `${(syncView.progress * 100).toFixed(1)}%` }" />
                        </div>
                    </div>
                </div>
                <div class="flex shrink-0 items-center">
                    <Button
                        v-if="playing"
                        type="colored"
                        color="red"
                        size="xl"
                        class="galaxy-play-button"
                        :disabled="stopping"
                        @click="stop()"
                        ><StopCircleIcon />
                        {{ stopping ? TEXT.stopping : TEXT.stop }}
                    </Button>
                    <Button
                        v-else-if="launching"
                        type="colored"
                        color="brand"
                        size="xl"
                        class="galaxy-play-button"
                        disabled
                        ><SpinnerIcon class="animate-spin" />
                        {{ fill(TEXT.starting, { seconds: launchElapsedSeconds }) }}
                    </Button>
                    <Button
                        v-else-if="galaxy.ready && galaxy.syncBusy"
                        type="colored"
                        color="brand"
                        size="xl"
                        class="galaxy-play-button"
                        disabled
                        ><SpinnerIcon class="animate-spin" />
                        {{ TEXT.syncUpdating }}
                    </Button>
                    <Button
                        v-else
                        v-tooltip="galaxy.syncBlocksPlay ? TEXT.syncPlayBlocked : undefined"
                        type="colored"
                        color="brand"
                        size="xl"
                        class="galaxy-play-button"
                        :disabled="!galaxy.ready || galaxy.syncBlocksPlay"
                        @click="play()"
                        ><SpinnerIcon v-if="galaxy.loading" class="animate-spin" />
                        <PlayIcon v-else />
                        {{ TEXT.play }}
                    </Button>
                </div>
            </div>
        </div>
    </div>

    <ConfirmModal
        ref="repairModal"
        :title="TEXT.repairTitle"
        :description="TEXT.repairDescription"
        :proceed-label="TEXT.repairProceed"
        :proceed-icon="WrenchIcon"
        :danger="false"
        :markdown="false"
        @proceed="repairGameFiles()"
    />

    <GalaxyPlayerNameModal ref="nameModal" variant="play" @saved="onNameSaved()" />

    <GalaxyGraphicsModal ref="graphicsModal" :playing="playing || launching" />

    <NewModal ref="syncDetailsModal" :header="TEXT.syncDetailsTitle" max-width="34rem">
        <div class="flex flex-col gap-4">
            <div class="flex flex-col gap-2">
                <div class="galaxy-info-row">
                    <span class="galaxy-info-label">{{ TEXT.syncDetailsCurrent }}</span>
                    <span class="text-sm font-semibold text-[var(--color-text-primary)]">{{
                        syncDetails.version
                    }}</span>
                </div>
                <div v-if="syncDetails.releasedAt" class="galaxy-info-row">
                    <span class="galaxy-info-label">{{ TEXT.syncDetailsReleased }}</span>
                    <span class="text-sm text-[var(--color-text-default)]">{{
                        syncDetails.releasedAt
                    }}</span>
                </div>
            </div>
            <div class="flex flex-col gap-1.5">
                <h3 class="m-0 text-sm font-bold text-[var(--color-text-primary)]">
                    {{ TEXT.syncDetailsChangelog }}
                </h3>
                <p
                    class="m-0 whitespace-pre-wrap text-sm leading-6"
                    :class="
                        syncDetails.changelog
                            ? 'text-[var(--color-text-default)]'
                            : 'text-[var(--color-text-tertiary)]'
                    "
                >
                    {{ syncDetails.changelog || TEXT.syncDetailsNoChangelog }}
                </p>
            </div>
            <div v-if="syncDetails.history.length" class="flex flex-col gap-1.5">
                <h3 class="m-0 text-sm font-bold text-[var(--color-text-primary)]">
                    {{ TEXT.syncDetailsHistory }}
                </h3>
                <ul class="galaxy-sync-history">
                    <li v-for="entry in syncDetails.history" :key="entry.releaseId">
                        <div class="flex items-center gap-2">
                            <span
                                class="galaxy-badge"
                                :class="{ 'is-pinned': entry.releaseId === syncDetails.releaseId }"
                                >{{ entry.displayVersion || `#${entry.sequence}` }}</span
                            >
                            <time class="text-xs text-[var(--color-text-tertiary)]">{{
                                formatReleaseTime(entry.createdAt)
                            }}</time>
                        </div>
                        <p
                            v-if="entry.changelog"
                            class="m-0 whitespace-pre-wrap text-sm text-[var(--color-text-default)]"
                        >
                            {{ entry.changelog }}
                        </p>
                    </li>
                </ul>
            </div>
        </div>
        <template #actions>
            <div class="flex justify-end">
                <Button @click="syncDetailsModal?.hide()">{{ TEXT.syncDetailsClose }}</Button>
            </div>
        </template>
    </NewModal>
</template>

<style scoped>
/* Modrinth light UI: white cards with a hairline border and a soft shadow.
 * Colours come from theme tokens (the hero uses brand green shades), so the
 * dark themes keep working. */
.galaxy-hero {
    min-height: 13rem;
    display: flex;
    align-items: center;
    background:
        radial-gradient(
            circle at 88% 20%,
            color-mix(in srgb, white 18%, transparent) 0 7rem,
            transparent 7rem
        ),
        radial-gradient(
            circle at 78% 115%,
            color-mix(in srgb, white 10%, transparent) 0 11rem,
            transparent 11rem
        ),
        linear-gradient(
            120deg,
            color-mix(in srgb, var(--color-brand) 80%, black) 0%,
            var(--color-brand) 45%,
            color-mix(in srgb, var(--color-brand) 55%, #0e7490) 100%
        );
    box-shadow: var(--shadow-card);
}

.galaxy-hero > .relative {
    width: 100%;
}

.galaxy-hero-pattern {
    position: absolute;
    inset: 0;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='48' height='48' viewBox='0 0 48 48'%3E%3Cpath d='M24 4 44 24 24 44 4 24Z' fill='none' stroke='white' stroke-opacity='0.13' stroke-width='1.5'/%3E%3Ccircle cx='24' cy='24' r='2' fill='white' fill-opacity='0.16'/%3E%3C/svg%3E");
    background-size: 48px 48px;
    mask-image: linear-gradient(90deg, transparent 0%, black 55%, black 100%);
    -webkit-mask-image: linear-gradient(90deg, transparent 0%, black 55%, black 100%);
}

.galaxy-hero-icon {
    display: flex;
    width: 7rem;
    height: 7rem;
    align-items: center;
    justify-content: center;
    padding: 0.75rem;
    border: 1px solid color-mix(in srgb, white 45%, transparent);
    border-radius: 1.5rem;
    background: color-mix(in srgb, white 92%, transparent);
    box-shadow: 0 10px 30px color-mix(in srgb, black 22%, transparent);
}

.galaxy-hero-icon img {
    width: 100%;
    height: 100%;
    object-fit: contain;
}

.galaxy-hero-icon img.pixelated {
    image-rendering: pixelated;
}

.galaxy-hero-motd {
    max-width: 100%;
    padding: 0.4rem 0.75rem;
    border-radius: 0.75rem;
    background: color-mix(in srgb, black 30%, transparent);
    color: white;
}

.galaxy-hero-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.3rem 0.75rem;
    border: 1px solid color-mix(in srgb, white 30%, transparent);
    border-radius: 9999px;
    background: color-mix(in srgb, white 16%, transparent);
    color: white;
    font-size: 0.8125rem;
    font-weight: 600;
    white-space: nowrap;
}

.galaxy-hero-chip :deep(svg) {
    width: 0.95rem;
    height: 0.95rem;
}

.galaxy-online-dot {
    width: 0.5rem;
    height: 0.5rem;
    border-radius: 9999px;
    background: white;
    box-shadow: 0 0 0 3px color-mix(in srgb, white 35%, transparent);
}

.galaxy-hero-chip.is-maintenance {
    border-color: color-mix(in srgb, #fbbf24 70%, transparent);
    background: color-mix(in srgb, #f59e0b 55%, transparent);
}

/* Online players of the main server (data API), vanilla default heads by name. */
.galaxy-online-heads {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(4.25rem, 1fr));
    gap: 0.5rem 0.35rem;
    margin: 0;
    padding: 0;
    list-style: none;
}

.galaxy-online-heads li {
    display: flex;
    min-width: 0;
    flex-direction: column;
    align-items: center;
    gap: 0.2rem;
}

.galaxy-online-heads img,
.galaxy-online-head-fallback {
    width: 2rem;
    height: 2rem;
    border-radius: 0.4rem;
    box-shadow: 0 1px 3px color-mix(in srgb, black 18%, transparent);
}

.galaxy-online-heads img.pixelated {
    image-rendering: pixelated;
}

.galaxy-online-head-fallback {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: var(--surface-2);
    color: var(--color-text-tertiary);
    font-size: 0.8rem;
    font-weight: 700;
}

.galaxy-online-name {
    max-width: 100%;
    overflow: hidden;
    color: var(--color-text-default);
    font-size: 0.7rem;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.galaxy-lobby-grid {
    display: grid;
    grid-template-columns: minmax(0, 2fr) minmax(17rem, 1fr);
    align-items: start;
    gap: 1.25rem;
}

@media (max-width: 960px) {
    .galaxy-lobby-grid {
        grid-template-columns: minmax(0, 1fr);
    }
}

.galaxy-section-title {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--color-brand);
}

.galaxy-section-title :deep(svg) {
    width: 1.15rem;
    height: 1.15rem;
}

.galaxy-section-title h2 {
    margin: 0;
    color: var(--color-text-primary);
    font-size: 1.05rem;
    font-weight: 700;
}

.galaxy-card {
    border: 1px solid var(--surface-5);
    border-radius: 1rem;
    background: var(--surface-4);
    box-shadow: var(--shadow-card);
}

.galaxy-address,
.galaxy-badge {
    border: 1px solid var(--surface-5);
    background: var(--surface-2);
    color: var(--color-text-default);
}

.galaxy-address {
    min-width: 0;
    padding: 0.15rem 0.5rem;
    border-radius: 0.5rem;
    font-size: 0.8rem;
}

.galaxy-badge {
    display: inline-flex;
    align-items: center;
    padding: 0.1rem 0.5rem;
    border-radius: 9999px;
    font-size: 0.75rem;
    font-weight: 600;
}

.galaxy-badge.is-pinned {
    border-color: color-mix(in srgb, var(--color-brand) 35%, transparent);
    background: var(--color-brand-highlight);
    color: var(--color-brand);
}

.galaxy-badge.is-must-read {
    border-color: color-mix(in srgb, var(--color-red) 30%, transparent);
    background: color-mix(in srgb, var(--color-red) 9%, transparent);
    color: var(--color-red);
}

.galaxy-view-all {
    display: inline-flex;
    align-items: center;
    gap: 0.15rem;
    margin-left: auto;
    color: var(--color-brand);
    font-size: 0.8125rem;
    font-weight: 600;
    text-decoration: none;
}

.galaxy-view-all:hover {
    text-decoration: underline;
}

.galaxy-section-title .galaxy-view-all :deep(svg) {
    width: 1rem;
    height: 1rem;
}

.galaxy-announcement {
    color: inherit;
    text-decoration: none;
    transition:
        border-color 150ms ease,
        box-shadow 150ms ease;
}

.galaxy-announcement:hover {
    border-color: color-mix(in srgb, var(--color-brand) 45%, var(--surface-5));
}

.galaxy-announcement:focus-visible,
.galaxy-view-all:focus-visible {
    outline: 2px solid var(--color-brand);
    outline-offset: 2px;
}

.galaxy-announcement-preview {
    display: -webkit-box;
    overflow: hidden;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
}

.galaxy-info-row {
    display: flex;
    min-width: 0;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
}

.galaxy-info-label {
    flex-shrink: 0;
    color: var(--color-text-tertiary);
    font-size: 0.8125rem;
}

.galaxy-icon-button {
    display: inline-flex;
    width: 1.75rem;
    height: 1.75rem;
    flex-shrink: 0;
    align-items: center;
    justify-content: center;
    padding: 0;
    border: 0;
    border-radius: 9999px;
    background: transparent;
    color: var(--color-text-tertiary);
    cursor: pointer;
}

.galaxy-icon-button:hover:not(:disabled) {
    background: var(--surface-2);
    color: var(--color-text-primary);
}

.galaxy-icon-button :deep(svg) {
    width: 1rem;
    height: 1rem;
}

.galaxy-action {
    display: flex;
    width: 100%;
    min-height: 2.75rem;
    align-items: center;
    gap: 0.75rem;
    padding: 0.5rem 0.75rem;
    border: 0;
    border-radius: 0.75rem;
    background: transparent;
    color: var(--color-text-primary);
    font-size: 0.875rem;
    font-weight: 600;
    text-align: left;
    cursor: pointer;
}

.galaxy-action :deep(svg) {
    width: 1.15rem;
    height: 1.15rem;
    flex-shrink: 0;
    color: var(--color-text-tertiary);
}

.galaxy-action:hover:not(:disabled) {
    background: var(--surface-2);
}

.galaxy-action:hover:not(:disabled) :deep(svg) {
    color: var(--color-brand);
}

.galaxy-action:disabled {
    cursor: not-allowed;
    opacity: 0.5;
}

.galaxy-action:focus-visible,
.galaxy-link:focus-visible,
.galaxy-icon-button:focus-visible {
    outline: 2px solid var(--color-brand);
    outline-offset: 2px;
}

/* The dock sits in normal flow at the end of the page and sticks to the
 * bottom of the scroll area, so it never covers the last card. */
.galaxy-dock-wrap {
    position: sticky;
    bottom: 0;
    z-index: 5;
    margin-top: auto;
    padding: 0.5rem 0 1.25rem;
    background: linear-gradient(to bottom, transparent, var(--color-bg) 45%);
}

.galaxy-dock {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.75rem 0.75rem 0.75rem 1rem;
    border: 1px solid var(--surface-5);
    border-radius: 1.25rem;
    background: var(--surface-4);
    box-shadow:
        0 -2px 12px color-mix(in srgb, black 6%, transparent),
        var(--shadow-card);
}

.galaxy-dock-placeholder {
    display: inline-flex;
    width: 40px;
    height: 40px;
    align-items: center;
    justify-content: center;
    border-radius: 0.75rem;
    background: var(--surface-2);
    color: var(--color-text-tertiary);
}

.galaxy-dock-placeholder :deep(svg) {
    width: 1.25rem;
    height: 1.25rem;
}

/* Modpack sync status in the middle of the dock (built-in updater). */
.galaxy-sync {
    display: flex;
    min-width: 0;
    max-width: 32rem;
    flex: 1 1 auto;
    flex-direction: column;
    gap: 0.4rem;
    font-size: 0.875rem;
}

.galaxy-sync-line {
    display: flex;
    min-width: 0;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
}

.galaxy-sync-line > :deep(svg) {
    width: 1rem;
    height: 1rem;
    flex-shrink: 0;
}

.galaxy-sync-text {
    min-width: 0;
    overflow: hidden;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
}

button.galaxy-sync-text {
    padding: 0;
    border: 0;
    background: transparent;
    color: inherit;
    font: inherit;
    cursor: pointer;
}

button.galaxy-sync-text:hover {
    text-decoration: underline;
}

button.galaxy-sync-text:focus-visible {
    outline: 2px solid var(--color-brand);
    outline-offset: 2px;
}

.galaxy-sync-action {
    flex-shrink: 0;
    align-self: center;
}

.galaxy-sync.is-neutral {
    color: var(--color-text-default);
}

.galaxy-sync.is-ok {
    color: var(--color-brand);
}

.galaxy-sync.is-warn {
    color: var(--color-orange);
}

.galaxy-sync.is-danger {
    color: var(--color-red);
}

.galaxy-sync-bar {
    height: 0.375rem;
    overflow: hidden;
    border-radius: 9999px;
    background: var(--surface-2);
}

.galaxy-sync-bar > span {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: var(--color-brand);
    transition: width 200ms ease;
}

.galaxy-sync-history {
    display: flex;
    max-height: 16rem;
    flex-direction: column;
    gap: 0.6rem;
    margin: 0;
    padding: 0;
    overflow-y: auto;
    list-style: none;
}

.galaxy-sync-history li {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    padding-bottom: 0.6rem;
    border-bottom: 1px solid var(--surface-5);
}

.galaxy-sync-history li:last-child {
    padding-bottom: 0;
    border-bottom: 0;
}

.galaxy-link {
    align-self: flex-start;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--color-brand);
    font-size: 0.8125rem;
    font-weight: 600;
    cursor: pointer;
}

.galaxy-link:hover {
    text-decoration: underline;
}

.galaxy-play-button {
    min-width: 11rem;
}
</style>
