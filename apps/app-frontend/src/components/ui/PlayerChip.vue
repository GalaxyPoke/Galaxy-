<script setup lang="ts">
/**
 * Galaxy宝可梦 header player chip: skin head + current game name. Clicking it
 * opens a popout with the default slot (App.vue puts the single AccountsCard
 * there). The popout uses `v-show`, never `v-if`, so the AccountsCard stays
 * mounted and the `accountsCard` ref provided by App.vue keeps working.
 * Without any account the chip asks for a name instead (`set-name`).
 */
import { DropdownIcon, UserPlusIcon } from '@modrinth/assets'
import { Avatar } from '@modrinth/ui'
import { onBeforeUnmount, onMounted, ref } from 'vue'

const props = defineProps<{
    name?: string | null
    avatarUrl?: string | null
    hasAccounts?: boolean
}>()

const emit = defineEmits<{
    'set-name': []
}>()

const TEXT = {
    setName: '设置游戏名',
    switchHint: '切换游戏名',
    title: '当前游玩账号',
}

const open = ref(false)
const root = ref<HTMLElement | null>(null)

function toggle() {
    if (!props.hasAccounts) {
        open.value = false
        emit('set-name')
        return
    }
    open.value = !open.value
}

function close() {
    open.value = false
}

function onPointerDown(event: PointerEvent) {
    if (!open.value) return
    const target = event.target as Node | null
    if (target && root.value?.contains(target)) return
    open.value = false
}

function onKeydown(event: KeyboardEvent) {
    if (open.value && event.key === 'Escape') open.value = false
}

onMounted(() => {
    document.addEventListener('pointerdown', onPointerDown, true)
    document.addEventListener('keydown', onKeydown)
})

onBeforeUnmount(() => {
    document.removeEventListener('pointerdown', onPointerDown, true)
    document.removeEventListener('keydown', onKeydown)
})

function show() {
    if (!props.hasAccounts) {
        emit('set-name')
        return
    }
    open.value = true
}

defineExpose({ close, show })
</script>

<template>
    <div ref="root" class="relative flex" data-tauri-drag-region-exclude>
        <button
            v-tooltip="hasAccounts ? TEXT.switchHint : undefined"
            type="button"
            class="player-chip"
            :class="{ 'is-open': open, 'is-empty': !name }"
            data-onboarding-id="offline-account-entry"
            :aria-expanded="hasAccounts ? open : undefined"
            aria-haspopup="true"
            @click="toggle"
        >
            <Avatar
                v-if="name && avatarUrl"
                :src="avatarUrl"
                size="26px"
                pixelated
                :unframed-natural-width="72"
                class="shrink-0"
            />
            <UserPlusIcon v-else class="size-5 shrink-0" aria-hidden="true" />
            <span class="max-w-[9rem] truncate">{{ name || TEXT.setName }}</span>
            <DropdownIcon
                v-if="hasAccounts"
                class="size-4 shrink-0 text-[var(--color-text-tertiary)] transition-transform"
                :class="{ 'rotate-180': open }"
                aria-hidden="true"
            />
        </button>
        <div v-show="open" class="player-popout" role="dialog" :aria-label="TEXT.title">
            <div class="px-1 text-sm font-semibold text-[var(--color-text-primary)]">
                {{ TEXT.title }}
            </div>
            <slot />
        </div>
    </div>
</template>

<style scoped>
.player-chip {
    display: inline-flex;
    height: 2.25rem;
    align-items: center;
    gap: 0.5rem;
    padding: 0 0.6rem 0 0.3rem;
    border: 1px solid var(--surface-5);
    border-radius: 9999px;
    background: var(--surface-4);
    color: var(--color-text-primary);
    font-size: 0.875rem;
    font-weight: 600;
    cursor: pointer;
    transition:
        background-color 150ms ease,
        border-color 150ms ease;
}

.player-chip.is-empty {
    padding-left: 0.6rem;
    border-color: var(--color-brand);
    color: var(--color-brand);
}

.player-chip:hover,
.player-chip.is-open {
    background: var(--surface-3);
}

.player-chip:focus-visible {
    outline: 2px solid var(--color-brand);
    outline-offset: 2px;
}

.player-chip :deep(.avatar),
.player-chip :deep(img) {
    border-radius: 0.4rem;
}

.player-popout {
    position: absolute;
    top: calc(100% + 0.5rem);
    right: 0;
    z-index: 300;
    display: flex;
    width: 20rem;
    max-width: calc(100vw - 2rem);
    flex-direction: column;
    gap: 0.25rem;
    padding: 0.75rem;
    border: 1px solid var(--surface-5);
    border-radius: var(--radius-lg, 1rem);
    background: var(--surface-3);
    box-shadow: var(--shadow-floating, 0 12px 32px rgba(0, 0, 0, 0.16));
}
</style>
