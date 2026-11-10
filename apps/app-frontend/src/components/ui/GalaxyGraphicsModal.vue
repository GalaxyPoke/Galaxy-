<script setup lang="ts">
/**
 * 画质档位: one-click graphics presets (backend galaxy_graphics.rs rewrites
 * options.txt and the Iris shader switch). Own settings are saved before the
 * first switch and can be put back.
 */
import { CheckIcon, SpinnerIcon } from '@modrinth/assets'
import { Button, injectNotificationManager, NewModal } from '@modrinth/ui'
import { computed, ref } from 'vue'

import {
    galaxyErrorText,
    galaxyGraphicsApply,
    galaxyGraphicsGet,
    type GalaxyGraphicsPreset,
    type GalaxyGraphicsState,
} from '@/helpers/galaxy'

const props = defineProps<{
    /** The game is running: switching is refused (it writes its settings back). */
    playing: boolean
}>()

const { addNotification } = injectNotificationManager()
const modal = ref<InstanceType<typeof NewModal> | null>(null)
const state = ref<GalaxyGraphicsState | null>(null)
const loadError = ref<string | null>(null)
const applying = ref<GalaxyGraphicsPreset | 'custom' | null>(null)

interface Option {
    id: GalaxyGraphicsPreset
    title: string
    text: string
}

const options = computed<Option[]>(() => {
    const pack = state.value?.shaderPack?.replace(/\.zip$/i, '')
    const shaderText = state.value?.shaderPackAvailable
        ? `开启光影${pack ? `（${pack}）` : ''}`
        : '没找到光影包，只调高画质'
    return [
        {
            id: 'low',
            title: '低配流畅',
            text: '核显、老电脑或掉帧时用：视距 6、快速画质、粒子最少、关闭云和光影。',
        },
        {
            id: 'balanced',
            title: '均衡（推荐）',
            text: '大多数电脑：视距 8、高品质画质、粒子减少、关闭光影。',
        },
        {
            id: 'high',
            title: '高画质',
            text: `独立显卡（GTX 1660 / RX 580 以上）：视距 10、全部粒子、${shaderText}。`,
        },
    ]
})

const PRESET_NAMES: Record<GalaxyGraphicsPreset, string> = {
    low: '低配流畅',
    balanced: '均衡',
    high: '高画质',
}

const currentText = computed(() => {
    const current = state.value
    if (!current) return ''
    if (!current.optionsFound) return '还没有游戏设置文件，先启动一次游戏再来切换。'
    if (current.preset) return `当前：${PRESET_NAMES[current.preset]}`
    return '当前：自己调过的设置'
})

async function load() {
    loadError.value = null
    try {
        state.value = await galaxyGraphicsGet()
    } catch (error) {
        state.value = null
        loadError.value = galaxyErrorText(error)
    }
}

async function show() {
    state.value = null
    modal.value?.show()
    await load()
}

async function apply(preset: GalaxyGraphicsPreset | 'custom') {
    if (props.playing || applying.value) return
    applying.value = preset
    try {
        state.value = await galaxyGraphicsApply(preset)
        addNotification({
            type: 'success',
            title:
                preset === 'custom' ? '已恢复你自己的设置' : `已切换到「${PRESET_NAMES[preset]}」`,
            text: '下次进入游戏时生效。游戏里也可以继续微调（选项 → 视频设置）。',
        })
    } catch (error) {
        addNotification({ type: 'error', title: '切换画质失败', text: galaxyErrorText(error) })
        await load()
    } finally {
        applying.value = null
    }
}

defineExpose({ show })
</script>

<template>
    <NewModal ref="modal" header="画质档位" max-width="34rem">
        <div class="flex flex-col gap-3">
            <p v-if="playing" class="galaxy-graphics-notice">
                游戏正在运行，关闭游戏后才能切换画质。
            </p>
            <p v-else-if="loadError" class="galaxy-graphics-notice is-error">{{ loadError }}</p>
            <p v-else-if="currentText" class="m-0 text-sm text-[var(--color-text-tertiary)]">
                {{ currentText }}
            </p>
            <button
                v-for="option in options"
                :key="option.id"
                type="button"
                class="galaxy-graphics-option"
                :class="{ 'is-current': state?.preset === option.id }"
                :disabled="playing || !state?.optionsFound || applying !== null"
                @click="apply(option.id)"
            >
                <span class="flex min-w-0 flex-1 flex-col gap-0.5 text-left">
                    <span class="galaxy-graphics-title">{{ option.title }}</span>
                    <span class="galaxy-graphics-text">{{ option.text }}</span>
                </span>
                <SpinnerIcon
                    v-if="applying === option.id"
                    class="animate-spin"
                    aria-hidden="true"
                />
                <CheckIcon
                    v-else-if="state?.preset === option.id"
                    class="text-brand"
                    aria-hidden="true"
                />
            </button>
            <button
                v-if="state?.hasBackup"
                type="button"
                class="galaxy-graphics-restore"
                :disabled="playing || applying !== null"
                @click="apply('custom')"
            >
                <SpinnerIcon v-if="applying === 'custom'" class="animate-spin" aria-hidden="true" />
                恢复我自己调过的设置
            </button>
        </div>
        <template #actions>
            <div class="flex justify-end">
                <Button @click="modal?.hide()">关闭</Button>
            </div>
        </template>
    </NewModal>
</template>

<style scoped>
.galaxy-graphics-notice {
    margin: 0;
    padding: 0.5rem 0.75rem;
    border-radius: 0.65rem;
    background: color-mix(in srgb, var(--color-orange) 10%, transparent);
    color: var(--color-orange);
    font-size: 0.8125rem;
    font-weight: 600;
}

.galaxy-graphics-notice.is-error {
    background: color-mix(in srgb, var(--color-red) 9%, transparent);
    color: var(--color-red);
}

.galaxy-graphics-option {
    display: flex;
    width: 100%;
    align-items: center;
    gap: 0.75rem;
    padding: 0.75rem 0.9rem;
    border: 1px solid var(--surface-5);
    border-radius: 0.85rem;
    background: var(--surface-2);
    color: inherit;
    cursor: pointer;
}

.galaxy-graphics-option:hover:not(:disabled) {
    border-color: color-mix(in srgb, var(--color-brand) 55%, var(--surface-5));
}

.galaxy-graphics-option.is-current {
    border-color: var(--color-brand);
    background: var(--color-brand-highlight);
}

.galaxy-graphics-option:disabled {
    cursor: not-allowed;
    opacity: 0.6;
}

.galaxy-graphics-option :deep(svg) {
    width: 1.15rem;
    height: 1.15rem;
    flex-shrink: 0;
}

.galaxy-graphics-title {
    color: var(--color-text-primary);
    font-size: 0.9rem;
    font-weight: 700;
}

.galaxy-graphics-text {
    color: var(--color-text-default);
    font-size: 0.8rem;
    line-height: 1.45;
}

.galaxy-graphics-restore {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    align-self: flex-start;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--color-brand);
    font-size: 0.8125rem;
    font-weight: 600;
    cursor: pointer;
}

.galaxy-graphics-restore:disabled {
    cursor: not-allowed;
    opacity: 0.5;
}

.galaxy-graphics-restore :deep(svg) {
    width: 0.95rem;
    height: 0.95rem;
}

.galaxy-graphics-option:focus-visible,
.galaxy-graphics-restore:focus-visible {
    outline: 2px solid var(--color-brand);
    outline-offset: 2px;
}
</style>
