import {
    provideI18n,
    provideNotificationManager,
    providePopupNotificationManager,
} from '@modrinth/ui'
import { createPinia } from 'pinia'
import { afterEach, expect, it, vi } from 'vitest'
import { page } from 'vitest/browser'
import { computed, defineComponent, h, nextTick, ref, Suspense } from 'vue'
import { createMemoryHistory, createRouter } from 'vue-router'

import { mountThemed, waitFor } from '@/test/visual-harness'

import AppActionBar from './AppActionBar.vue'
import AppTitlebar from './AppTitlebar.vue'

const fixture = vi.hoisted(() => ({
    instanceName: '我的整合包实例名称非常长'.repeat(20),
    processes: [] as { uuid: string; instance_id: string }[],
}))

vi.mock('@/helpers/process', () => ({
    get_all: async () => fixture.processes,
    kill: vi.fn(),
}))
vi.mock('@/helpers/instance', () => ({
    get_many: async () => [{ id: 'demo', name: fixture.instanceName }],
}))
vi.mock('@/helpers/events', () => ({
    loading_listener: async () => vi.fn(),
    process_listener: async () => vi.fn(),
}))
vi.mock('@/helpers/instance-backup', () => ({
    listBackupOperations: async () => [],
    listenBackupProgress: async () => vi.fn(),
}))
vi.mock('@/helpers/state', () => ({ progress_bars_list: async () => ({}) }))
vi.mock('@/helpers/analytics', () => ({ trackEvent: vi.fn() }))
vi.mock('@/helpers/remote-announcements', () => ({
    OPEN_REMOTE_ANNOUNCEMENT_CENTER_EVENT: 'open-announcements',
    REMOTE_ANNOUNCEMENTS_UPDATED_EVENT: 'announcements-updated',
}))
vi.mock('@/composables/useNetworkStatus', () => ({
    useNetworkStatus: () => ({ offline: ref(false) }),
}))
vi.mock('@/providers/download-manager', () => ({
    injectDownloadManager: () => ({
        activeJobs: ref([]),
        legacyDownloads: ref([]),
        cancel: vi.fn(),
    }),
}))
vi.mock(import('@/composables/browse/install-job-notifications'), () => ({
    useInstallJobNotifications: async () => ({
        active: computed(() => false),
        hasItems: computed(() => false),
        title: computed(() => 'Installs'),
        progressItems: computed(() => []),
        stateSignature: computed(() => ''),
        buttons: computed(() => undefined),
        refresh: vi.fn(async () => {}),
        dispose: vi.fn(),
    }),
}))
vi.mock('@/providers/app-update', () => ({
    appUpdateState: {
        downloading: ref(false),
        downloadPercent: ref(0),
        downloadProgress: ref(0),
        finishedDownloading: ref(false),
        isVisible: ref(false),
        metered: ref(false),
        restarting: ref(false),
    },
    downloadAvailableAppUpdate: vi.fn(),
    installAvailableAppUpdate: vi.fn(),
}))

const cleanup: (() => void)[] = []
afterEach(() => {
    cleanup.splice(0).forEach((dispose) => dispose())
    fixture.processes = []
    document.documentElement.classList.remove('accent-green')
})

const empty = { render: () => null }

async function mountTitlebar({
    withActionBar = false,
    gameDisabled = false,
    path = '/',
}: { withActionBar?: boolean; gameDisabled?: boolean; path?: string } = {}) {
    await page.viewport(1100, 720)
    const router = createRouter({
        history: createMemoryHistory(),
        routes: [
            { path: '/', component: empty },
            { path: '/instance/:id', component: empty },
            { path: '/instance/:id/logs', component: empty },
            { path: '/skins', component: empty },
            { path: '/settings', component: empty },
        ],
    })
    await router.push(path)
    const width = ref(1100)
    const Harness = defineComponent({
        setup() {
            provideI18n({ locale: ref('en-US'), t: (key) => key, setLocale: () => {} })
            provideNotificationManager({
                getNotifications: () => [],
                handleError: vi.fn(),
            } as unknown as Parameters<typeof provideNotificationManager>[0])
            providePopupNotificationManager({
                getNotifications: () => [],
            } as unknown as Parameters<typeof providePopupNotificationManager>[0])
            return () =>
                h(
                    AppTitlebar,
                    {
                        gamePath: '/instance/demo',
                        gameDisabled,
                        style: { width: `${width.value}px`, height: '3.5rem' },
                    },
                    {
                        default: () =>
                            withActionBar
                                ? h('div', { class: 'flex min-w-0 mr-3' }, [
                                      h(Suspense, {}, { default: () => h(AppActionBar) }),
                                  ])
                                : h('div', { 'data-test': 'actions' }, 'Actions'),
                    },
                )
        },
    })
    // The app runs the light theme with the green accent (index.html / theme store).
    document.documentElement.classList.add('accent-green')
    const wrapper = await mountThemed(Harness, {}, 'light', {
        global: { plugins: [router, createPinia()], directives: { tooltip: () => {} } },
    })
    cleanup.push(() => wrapper.unmount())
    return { wrapper, router, width }
}

function tabByOnboardingId(id: string): HTMLElement {
    const element = document.querySelector<HTMLElement>(`[data-onboarding-id="${id}"]`)
    if (!element) throw new Error(`missing tab ${id}`)
    return element
}

function pill(): HTMLElement {
    const element = document.querySelector<HTMLElement>('.galaxy-tab-pill')
    if (!element) throw new Error('missing pill')
    return element
}

function pillCovers(tab: HTMLElement): boolean {
    const a = pill().getBoundingClientRect()
    const b = tab.getBoundingClientRect()
    return Math.abs(a.left - b.left) < 1 && Math.abs(a.width - b.width) < 1
}

it('renders the four page tabs and puts the green pill behind the active one', async () => {
    const { wrapper, router } = await mountTitlebar()
    const labels = wrapper.findAll('.galaxy-tab').map((tab) => tab.text())
    expect(labels).toEqual(['主页', '游戏', '皮肤', '设置'])
    for (const id of ['nav-home', 'nav-game', 'nav-skins', 'nav-settings']) {
        expect(tabByOnboardingId(id).isConnected).toBe(true)
    }

    const home = tabByOnboardingId('nav-home')
    await waitFor(() => getComputedStyle(pill()).opacity === '1' && pillCovers(home), {
        label: 'pill on 主页',
    })
    expect(home.getAttribute('aria-current')).toBe('page')
    expect(getComputedStyle(pill()).backgroundColor).toBe('rgb(27, 217, 106)')
    expect(getComputedStyle(home).color).toBe('rgb(0, 0, 0)')

    await router.push('/skins')
    const skins = tabByOnboardingId('nav-skins')
    await waitFor(() => pillCovers(skins), { label: 'pill on 皮肤' })
    expect(skins.getAttribute('aria-current')).toBe('page')
    expect(home.getAttribute('aria-current')).toBeNull()

    // Instance sub-pages keep 游戏 active.
    await router.push('/instance/demo/logs')
    const game = tabByOnboardingId('nav-game')
    await waitFor(() => pillCovers(game), { label: 'pill on 游戏' })
    expect(game.getAttribute('href')).toBe('/instance/demo')
})

it('disables the 游戏 tab until the Galaxy instance exists', async () => {
    const { wrapper, router } = await mountTitlebar({ gameDisabled: true })
    const game = tabByOnboardingId('nav-game')
    expect(game.tagName).toBe('SPAN')
    expect(game.getAttribute('aria-disabled')).toBe('true')
    expect(game.getAttribute('href')).toBeNull()
    expect(Number(getComputedStyle(game).opacity)).toBeLessThan(1)
    await wrapper.get('[data-onboarding-id="nav-game"]').trigger('click')
    await nextTick()
    expect(router.currentRoute.value.path).toBe('/')
})

it('hides the running-game pill while idle', async () => {
    const { wrapper } = await mountTitlebar({ withActionBar: true })
    await waitFor(() => wrapper.findComponent(AppActionBar).exists())
    await new Promise((resolve) => setTimeout(resolve, 50))
    expect(wrapper.text()).not.toContain('游戏运行中')
    expect(wrapper.find('a[href="/instance/demo"][title]').exists()).toBe(false)
})

it('shows 游戏运行中 with stop and log buttons inside the header while the game runs', async () => {
    fixture.processes = [
        { uuid: 'process', instance_id: 'demo' },
        { uuid: 'process-2', instance_id: 'demo' },
    ]
    const { wrapper, width } = await mountTitlebar({ withActionBar: true })
    await waitFor(() => wrapper.find('a[href="/instance/demo"][title]').exists(), {
        label: 'running pill',
    })
    const link = wrapper.get('a[href="/instance/demo"][title]').element as HTMLElement
    expect(link.textContent?.trim()).toBe('游戏运行中')
    expect(link.title).toBe(fixture.instanceName)

    const header = wrapper.getComponent(AppTitlebar).element as HTMLElement
    for (const windowWidth of [1100, 1280]) {
        width.value = windowWidth
        await page.viewport(windowWidth, 720)
        await nextTick()
        expect(header.scrollWidth).toBeLessThanOrEqual(header.clientWidth)
        for (const action of wrapper.getComponent(AppActionBar).findAll('button')) {
            const rect = action.element.getBoundingClientRect()
            expect(rect.width).toBeGreaterThanOrEqual(20)
            expect(rect.right).toBeLessThanOrEqual(header.getBoundingClientRect().right)
            expect(
                action.element.contains(
                    document.elementFromPoint(rect.x + rect.width / 2, rect.y + rect.height / 2),
                ),
            ).toBe(true)
        }
    }
})

it('has no MCP connection control (Galaxy宝可梦 does not ship MCP)', async () => {
    const { wrapper } = await mountTitlebar()
    expect(wrapper.find('[data-testid="mcp-connection-status"]').exists()).toBe(false)
})
