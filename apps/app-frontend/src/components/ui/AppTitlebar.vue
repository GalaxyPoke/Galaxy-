<script setup lang="ts">
/**
 * Galaxy宝可梦 shell header (layout v2): logo + wordmark, the page tabs with an
 * animated pill, and a right-hand slot (action bar, player chip, window
 * controls). Replaces Axolotl's breadcrumbs / back-forward titlebar and the
 * left icon rail. The whole bar is a Tauri drag region; interactive children
 * opt out with `data-tauri-drag-region-exclude`.
 */
import { ChangeSkinIcon, GameIcon, HomeIcon, SettingsIcon } from '@modrinth/assets'
import { defineMessages, useVIntl } from '@modrinth/ui'
import { type Component, computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { RouterLink, useRoute } from 'vue-router'

// Galaxy宝可梦 (slimming phase 1): no McpConnectionStatus in the header; MCP is not offered.
// Restore: import McpConnectionStatus from '@/features/mcp/McpConnectionStatus.vue' and put
// <McpConnectionStatus /> back before the <slot /> below.
import GalaxyLogo from './GalaxyLogo.vue'

const props = defineProps<{
    isBetaBuild?: boolean
    gamePath: string
    gameDisabled?: boolean
}>()

const route = useRoute()
const { formatMessage } = useVIntl()
const messages = defineMessages({
    betaBuild: { id: 'app.build.beta', defaultMessage: 'Beta' },
})

type HeaderTab = {
    id: string
    label: string
    to: string
    icon: Component
    onboardingId: string
    disabled?: boolean
    active: (path: string) => boolean
}

const tabs = computed<HeaderTab[]>(() => [
    {
        id: 'home',
        label: '主页',
        to: '/',
        icon: HomeIcon,
        onboardingId: 'nav-home',
        active: (path) => path === '/' || path.startsWith('/announcements'),
    },
    {
        id: 'game',
        label: '游戏',
        to: props.gamePath,
        icon: GameIcon,
        onboardingId: 'nav-game',
        disabled: props.gameDisabled,
        active: (path) => path.startsWith('/instance'),
    },
    {
        id: 'skins',
        label: '皮肤',
        to: '/skins',
        icon: ChangeSkinIcon,
        onboardingId: 'nav-skins',
        active: (path) => path.startsWith('/skins'),
    },
    {
        id: 'settings',
        label: '设置',
        to: '/settings',
        icon: SettingsIcon,
        onboardingId: 'nav-settings',
        active: (path) => path.startsWith('/settings'),
    },
])

const activeTabId = computed(() => tabs.value.find((tab) => tab.active(route.path))?.id ?? null)

// ---- animated pill ------------------------------------------------------------
const tabList = ref<HTMLElement | null>(null)
const pillLeft = ref(0)
const pillWidth = ref(0)
const pillVisible = ref(false)
const pillAnimated = ref(false)
let resizeObserver: ResizeObserver | undefined

function positionPill() {
    const container = tabList.value
    const active = container?.querySelector<HTMLElement>('[data-tab-active="true"]')
    if (!container || !active) {
        pillVisible.value = false
        return
    }
    pillLeft.value = active.offsetLeft
    pillWidth.value = active.offsetWidth
    if (!pillVisible.value) {
        pillVisible.value = true
        requestAnimationFrame(() => {
            pillAnimated.value = true
        })
    }
}

async function updatePill() {
    await nextTick()
    positionPill()
}

onMounted(() => {
    void updatePill()
    if (tabList.value && typeof ResizeObserver !== 'undefined') {
        resizeObserver = new ResizeObserver(() => positionPill())
        resizeObserver.observe(tabList.value)
    }
    void document.fonts?.ready.then(() => positionPill())
})

onBeforeUnmount(() => resizeObserver?.disconnect())

watch(activeTabId, () => void updatePill())
</script>

<template>
    <header data-tauri-drag-region class="galaxy-header flex min-w-0 items-center gap-4">
        <div data-tauri-drag-region class="flex h-full shrink-0 items-center gap-2.5 pl-4">
            <div class="h-7 w-7 shrink-0 pointer-events-none">
                <GalaxyLogo icon-only />
            </div>
            <span
                data-tauri-drag-region
                class="select-none whitespace-nowrap text-base font-bold text-[var(--color-text-primary)]"
                >Galaxy宝可梦</span
            >
            <span
                v-if="isBetaBuild"
                class="inline-flex shrink-0 rounded-full bg-blue-100 px-2 py-0.5 text-xs font-semibold leading-none text-blue-700"
            >
                {{ formatMessage(messages.betaBuild) }}
            </span>
        </div>
        <span class="galaxy-header-divider shrink-0" aria-hidden="true" />
        <nav
            ref="tabList"
            class="galaxy-tabs relative flex shrink-0 items-center gap-1"
            aria-label="主导航"
            data-tauri-drag-region-exclude
        >
            <div
                class="galaxy-tab-pill pointer-events-none absolute rounded-full"
                :class="{ 'galaxy-tab-pill-animated': pillAnimated }"
                :style="{
                    left: `${pillLeft}px`,
                    width: `${pillWidth}px`,
                    opacity: pillVisible ? 1 : 0,
                }"
                aria-hidden="true"
            />
            <template v-for="tab in tabs" :key="tab.id">
                <span
                    v-if="tab.disabled"
                    class="galaxy-tab is-disabled"
                    :data-onboarding-id="tab.onboardingId"
                    aria-disabled="true"
                >
                    <component :is="tab.icon" aria-hidden="true" />
                    {{ tab.label }}
                </span>
                <RouterLink
                    v-else
                    :to="tab.to"
                    class="galaxy-tab"
                    :class="{ 'is-active': activeTabId === tab.id }"
                    :data-tab-active="activeTabId === tab.id ? 'true' : undefined"
                    :data-onboarding-id="tab.onboardingId"
                    :aria-current="activeTabId === tab.id ? 'page' : undefined"
                >
                    <component :is="tab.icon" aria-hidden="true" />
                    {{ tab.label }}
                </RouterLink>
            </template>
        </nav>
        <section data-tauri-drag-region class="ml-auto flex min-w-0 items-center gap-2">
            <slot />
        </section>
    </header>
</template>

<style scoped>
.galaxy-header {
    border-bottom: 1px solid var(--surface-5);
}

.galaxy-header-divider {
    width: 1px;
    height: 1.5rem;
    background: var(--surface-5);
}

.galaxy-tab {
    position: relative;
    z-index: 1;
    display: inline-flex;
    height: 2.25rem;
    align-items: center;
    gap: 0.4rem;
    padding: 0 0.9rem;
    border-radius: 9999px;
    color: var(--color-text-default);
    font-size: 0.9rem;
    font-weight: 600;
    white-space: nowrap;
    text-decoration: none;
    user-select: none;
    transition:
        color 150ms ease,
        background-color 150ms ease;
}

.galaxy-tab :deep(svg) {
    width: 1.1rem;
    height: 1.1rem;
}

.galaxy-tab:not(.is-active):not(.is-disabled):hover {
    background-color: var(--surface-3);
    color: var(--color-text-primary);
}

.galaxy-tab:focus-visible {
    outline: 2px solid var(--color-brand);
    outline-offset: 2px;
}

.galaxy-tab.is-active {
    color: var(--color-brand-fill-contrast, var(--color-accent-contrast));
}

.galaxy-tab.is-disabled {
    cursor: not-allowed;
    opacity: 0.45;
}

.galaxy-tab-pill {
    top: 0;
    bottom: 0;
    z-index: 0;
    background-color: var(--color-brand-fill, var(--color-brand));
    box-shadow: 0 2px 8px color-mix(in srgb, var(--color-brand) 30%, transparent);
}

.galaxy-tab-pill-animated {
    transition:
        left 220ms cubic-bezier(0.4, 0, 0.2, 1),
        width 220ms cubic-bezier(0.4, 0, 0.2, 1),
        opacity 200ms ease;
}

@media (prefers-reduced-motion: reduce) {
    .galaxy-tab-pill-animated {
        transition: none;
    }
}
</style>
