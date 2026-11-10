<script setup lang="ts">
/**
 * Galaxy宝可梦 player-name dialog, shared by the home page (before the first
 * launch, `variant="play"`) and AccountsCard ("添加其他游戏名", `variant="add"`).
 *
 * It validates the name live (validateGalaxyPlayerName: ^[A-Za-z0-9_]{3,16}$),
 * offers the saved offline names as quick picks and calls galaxySetPlayer(name),
 * which creates or reuses the offline account and makes it active. On success
 * it closes and emits `saved`; the caller refreshes the account card (and the
 * home page then launches the game).
 */
import { CheckIcon, InfoIcon, SpinnerIcon, TriangleAlertIcon, XIcon } from '@modrinth/assets'
import { Button, NewModal } from '@modrinth/ui'
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'

import galaxyLogo from '@/assets/galaxy-logo.svg?url'
import { vanillaDefaultSkinForName } from '@/galaxy/default-skin'
import { findVanillaDefaultSkin, getPlayerHeadDataUrl } from '@/galaxy/player-head'
import { defaultSteveHeadUrl } from '@/galaxy/skin-head'
import { users } from '@/helpers/auth'
import {
    GALAXY_PLAYER_NAME_PATTERN,
    galaxyErrorText,
    galaxySetPlayer,
    validateGalaxyPlayerName,
} from '@/helpers/galaxy'
import type { Skin } from '@/helpers/skins'
import { get_available_skins } from '@/helpers/skins'

const props = withDefaults(
    defineProps<{
        /** `play`: before starting the game; `add`: from the account list. */
        variant?: 'play' | 'add'
        confirmLabel?: string
        /** Disables the dialog while the caller is busy. */
        disabled?: boolean
    }>(),
    { variant: 'add', confirmLabel: undefined, disabled: false },
)

const emit = defineEmits<{
    saved: [name: string]
    busy: [value: boolean]
}>()

const TEXT = {
    welcome: '欢迎来到 Galaxy宝可梦',
    welcomeHint: '开始冒险前，先给自己起一个游戏名吧',
    choose: '选择你的游戏名',
    chooseHint: '选一个已有的游戏名，或者输入一个新的',
    add: '添加游戏名',
    addHint: '添加后可以随时在右上角切换',
    label: '游戏名',
    placeholder: '例如 Ash_Ketchum',
    preview: '你在服务器里会显示为',
    previewEmpty: '输入后在这里预览',
    skinCaption: '默认形象',
    skinHint: '没换皮肤时，大家在游戏里看到的就是这个形象；可以在「皮肤」页换成自己的皮肤',
    ruleLength: '3–16 个字符',
    ruleChars: '只用英文字母、数字和下划线',
    ruleSaved: '已经保存过这个名字，可直接选用',
    ruleCase: '和已保存的「{name}」只有大小写不同，会被当成另一个玩家',
    explainTitle: '这是离线账户，不需要密码',
    explain:
        '名字就是你在服务器里的身份。换名字等于换一个新玩家，之前的进度不会跟过去，请记好你的游戏名。',
    savedTitle: '或选择已有的游戏名',
    cancel: '取消',
    savePlay: '保存并开始游戏',
    save: '保存',
    failed: '设置游戏名失败',
    close: '关闭',
}

const modal = ref<InstanceType<typeof NewModal> | null>(null)
const input = ref<HTMLInputElement | null>(null)
const name = ref('')
const submitting = ref(false)
const submitError = ref<string | null>(null)
const savedNames = ref<string[]>([])

/*
 * Head preview, like PCL: as the player types, show the vanilla default skin
 * that name gets in game (picked from its offline UUID).
 */
const defaultSkins = ref<Skin[]>([])
const previewHeadUrl = ref(defaultSteveHeadUrl)
const previewSkinName = ref<string | null>(null)
const savedHeads = ref<Record<string, string>>({})
let previewToken = 0
let previewTimer: ReturnType<typeof setTimeout> | undefined

async function headForName(value: string): Promise<string | null> {
    const skin = findVanillaDefaultSkin(defaultSkins.value, value)
    if (!skin) return null
    try {
        return await getPlayerHeadDataUrl(skin)
    } catch (error) {
        console.warn('Failed to render default head for', value, error)
        return null
    }
}

async function updatePreview(value: string) {
    const token = ++previewToken
    if (!value || !/^[A-Za-z0-9_]+$/.test(value)) {
        previewHeadUrl.value = defaultSteveHeadUrl
        previewSkinName.value = null
        return
    }
    const head = await headForName(value)
    if (token !== previewToken) return
    previewHeadUrl.value = head ?? defaultSteveHeadUrl
    previewSkinName.value = head ? vanillaDefaultSkinForName(value).name : null
}

async function updateSavedHeads() {
    const heads: Record<string, string> = {}
    for (const saved of savedNames.value) {
        const head = await headForName(saved)
        if (head) heads[saved] = head
    }
    savedHeads.value = heads
}

async function loadDefaultSkins() {
    try {
        const skins = await get_available_skins(true)
        defaultSkins.value = skins.filter((skin) => skin.source === 'default')
    } catch {
        defaultSkins.value = []
    }
}

const trimmed = computed(() => name.value.trim())

watch(trimmed, (value) => {
    clearTimeout(previewTimer)
    previewTimer = setTimeout(() => void updatePreview(value), 120)
})

onBeforeUnmount(() => clearTimeout(previewTimer))

const validation = computed(() => validateGalaxyPlayerName(name.value))
const valid = computed(() => validation.value === null)
const busy = computed(() => submitting.value || props.disabled)

type RuleState = 'idle' | 'ok' | 'fail'
const lengthRule = computed<RuleState>(() => {
    if (!trimmed.value) return 'idle'
    return trimmed.value.length >= 3 && trimmed.value.length <= 16 ? 'ok' : 'fail'
})
const charsRule = computed<RuleState>(() => {
    if (!trimmed.value) return 'idle'
    return /^[A-Za-z0-9_]+$/.test(trimmed.value) ? 'ok' : 'fail'
})
const savedMatch = computed(() => savedNames.value.includes(trimmed.value))
const caseMatch = computed(() => {
    if (!trimmed.value || savedMatch.value) return null
    const lower = trimmed.value.toLowerCase()
    return savedNames.value.find((saved) => saved.toLowerCase() === lower) ?? null
})

const title = computed(() => {
    if (props.variant === 'add') return TEXT.add
    return savedNames.value.length ? TEXT.choose : TEXT.welcome
})
const subtitle = computed(() => {
    if (props.variant === 'add') return TEXT.addHint
    return savedNames.value.length ? TEXT.chooseHint : TEXT.welcomeHint
})
const confirmText = computed(
    () => props.confirmLabel ?? (props.variant === 'play' ? TEXT.savePlay : TEXT.save),
)

type SavedAccount = { account_type?: string; profile?: { name?: string } }

async function loadSavedNames() {
    try {
        const list = (await users(true, true)) as unknown as SavedAccount[] | null
        savedNames.value = Array.isArray(list)
            ? list
                  .filter((account) => account.account_type === 'offline')
                  .map((account) => account.profile?.name ?? '')
                  .filter((saved) => GALAXY_PLAYER_NAME_PATTERN.test(saved))
            : []
    } catch {
        savedNames.value = []
    }
}

function focusInput() {
    void nextTick(() => {
        setTimeout(() => input.value?.focus(), 60)
    })
}

async function show() {
    name.value = ''
    submitError.value = null
    submitting.value = false
    previewHeadUrl.value = defaultSteveHeadUrl
    previewSkinName.value = null
    modal.value?.show()
    focusInput()
    await Promise.all([loadSavedNames(), defaultSkins.value.length ? null : loadDefaultSkins()])
    void updatePreview(trimmed.value)
    void updateSavedHeads()
}

function hide() {
    modal.value?.hide()
}

function pickSaved(saved: string) {
    name.value = saved
    submitError.value = null
    focusInput()
}

function onInput() {
    submitError.value = null
}

async function submit() {
    if (!valid.value || busy.value) return
    const value = trimmed.value
    submitting.value = true
    submitError.value = null
    emit('busy', true)
    try {
        await galaxySetPlayer(value)
        modal.value?.hide()
        emit('saved', value)
    } catch (error) {
        submitError.value = `${TEXT.failed}：${galaxyErrorText(error)}`
        focusInput()
    } finally {
        submitting.value = false
        emit('busy', false)
    }
}

defineExpose({ show, hide })
</script>

<template>
    <NewModal
        ref="modal"
        hide-header
        no-padding
        width="31rem"
        max-width="calc(100vw - 2rem)"
        :disable-close="submitting"
        :aria-label="title"
    >
        <div class="galaxy-name-dialog">
            <header class="galaxy-name-banner" data-tauri-drag-region>
                <div class="galaxy-name-banner-pattern" aria-hidden="true" />
                <div class="galaxy-name-banner-logo">
                    <img :src="galaxyLogo" alt="" />
                </div>
                <div class="relative flex min-w-0 flex-1 flex-col gap-0.5">
                    <h2 class="m-0 truncate text-xl font-extrabold leading-tight text-white">
                        {{ title }}
                    </h2>
                    <p class="galaxy-name-subtitle m-0 truncate text-sm font-medium">{{ subtitle }}</p>
                </div>
                <button
                    type="button"
                    class="galaxy-name-close"
                    :aria-label="TEXT.close"
                    :disabled="submitting"
                    @click="hide()"
                >
                    <XIcon aria-hidden="true" />
                </button>
            </header>

            <div class="flex flex-col gap-4 p-6">
                <div class="flex items-start gap-4">
                    <div class="galaxy-name-avatar-column">
                        <div class="galaxy-name-avatar" aria-hidden="true">
                            <img :src="previewHeadUrl" alt="" />
                        </div>
                        <span
                            v-if="previewSkinName"
                            v-tooltip="TEXT.skinHint"
                            class="galaxy-name-skin-caption"
                            >{{ TEXT.skinCaption }}<br />{{ previewSkinName }}</span
                        >
                    </div>
                    <div class="flex min-w-0 flex-1 flex-col gap-1.5">
                        <label
                            for="galaxy-player-name-input"
                            class="text-sm font-semibold text-[var(--color-text-primary)]"
                            >{{ TEXT.label }}</label
                        >
                        <div class="galaxy-name-field" :class="{ 'is-invalid': charsRule === 'fail' }">
                            <input
                                id="galaxy-player-name-input"
                                ref="input"
                                v-model="name"
                                type="text"
                                :placeholder="TEXT.placeholder"
                                :disabled="busy"
                                autocomplete="off"
                                autocapitalize="off"
                                spellcheck="false"
                                maxlength="16"
                                aria-describedby="galaxy-player-name-rules"
                                @input="onInput()"
                                @keydown.enter.prevent="submit()"
                            />
                            <span
                                class="galaxy-name-counter"
                                :class="{ 'is-full': trimmed.length >= 16 }"
                                aria-hidden="true"
                                >{{ trimmed.length }}/16</span
                            >
                        </div>
                        <div class="flex min-w-0 items-center gap-2 text-xs text-[var(--color-text-tertiary)]">
                            <span class="shrink-0">{{ TEXT.preview }}</span>
                            <span v-if="trimmed" class="galaxy-nametag truncate">{{ trimmed }}</span>
                            <span v-else class="italic">{{ TEXT.previewEmpty }}</span>
                        </div>
                    </div>
                </div>

                <ul id="galaxy-player-name-rules" class="galaxy-name-rules" aria-live="polite">
                    <li :class="`is-${lengthRule}`">
                        <XIcon v-if="lengthRule === 'fail'" aria-hidden="true" />
                        <CheckIcon v-else-if="lengthRule === 'ok'" aria-hidden="true" />
                        <span v-else class="galaxy-rule-dot" aria-hidden="true" />
                        {{ TEXT.ruleLength }}
                    </li>
                    <li :class="`is-${charsRule}`">
                        <XIcon v-if="charsRule === 'fail'" aria-hidden="true" />
                        <CheckIcon v-else-if="charsRule === 'ok'" aria-hidden="true" />
                        <span v-else class="galaxy-rule-dot" aria-hidden="true" />
                        {{ TEXT.ruleChars }}
                    </li>
                    <li v-if="savedMatch" class="is-saved">
                        <CheckIcon aria-hidden="true" />
                        {{ TEXT.ruleSaved }}
                    </li>
                    <li v-else-if="caseMatch" class="is-warn">
                        <TriangleAlertIcon aria-hidden="true" />
                        {{ TEXT.ruleCase.replace('{name}', caseMatch) }}
                    </li>
                </ul>

                <div v-if="savedNames.length" class="flex flex-col gap-2">
                    <span class="text-xs font-semibold text-[var(--color-text-tertiary)]">{{
                        TEXT.savedTitle
                    }}</span>
                    <div class="flex flex-wrap gap-2">
                        <button
                            v-for="saved in savedNames"
                            :key="saved"
                            type="button"
                            class="galaxy-name-chip"
                            :class="{ 'is-selected': saved === trimmed }"
                            :aria-pressed="saved === trimmed"
                            :disabled="busy"
                            @click="pickSaved(saved)"
                        >
                            <img :src="savedHeads[saved] ?? defaultSteveHeadUrl" alt="" />
                            {{ saved }}
                        </button>
                    </div>
                </div>

                <div class="galaxy-name-explain">
                    <InfoIcon aria-hidden="true" />
                    <div class="flex min-w-0 flex-col gap-0.5">
                        <span class="font-semibold text-[var(--color-text-primary)]">{{
                            TEXT.explainTitle
                        }}</span>
                        <span>{{ TEXT.explain }}</span>
                    </div>
                </div>

                <p v-if="submitError" class="galaxy-name-error" role="alert">
                    {{ submitError }}
                </p>

                <div class="flex items-center justify-end gap-2 pt-1">
                    <Button :disabled="submitting" @click="hide()">{{ TEXT.cancel }}</Button>
                    <Button
                        type="colored"
                        color="brand"
                        :disabled="busy || !valid"
                        @click="submit()"
                        ><SpinnerIcon v-if="submitting" class="animate-spin" />
                        <CheckIcon v-else />
                        {{ confirmText }}
                    </Button>
                </div>
            </div>
        </div>
    </NewModal>
</template>

<style scoped>
.galaxy-name-dialog {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border-radius: 1rem;
    background: var(--surface-3);
}

.galaxy-name-banner {
    position: relative;
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 1.25rem 1.25rem 1.25rem 1.5rem;
    overflow: hidden;
    background: linear-gradient(
        120deg,
        color-mix(in srgb, var(--color-brand) 80%, black) 0%,
        var(--color-brand) 50%,
        color-mix(in srgb, var(--color-brand) 55%, #0e7490) 100%
    );
}

.galaxy-name-banner-pattern {
    position: absolute;
    inset: 0;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='40' height='40' viewBox='0 0 40 40'%3E%3Cpath d='M20 3 37 20 20 37 3 20Z' fill='none' stroke='white' stroke-opacity='0.14' stroke-width='1.5'/%3E%3C/svg%3E");
    background-size: 40px 40px;
    mask-image: linear-gradient(90deg, transparent 10%, black 70%);
    -webkit-mask-image: linear-gradient(90deg, transparent 10%, black 70%);
}

.galaxy-name-subtitle {
    color: color-mix(in srgb, white 88%, transparent);
}

.galaxy-name-banner-logo {
    position: relative;
    display: flex;
    width: 3.25rem;
    height: 3.25rem;
    flex-shrink: 0;
    align-items: center;
    justify-content: center;
    padding: 0.4rem;
    border-radius: 0.9rem;
    background: color-mix(in srgb, white 94%, transparent);
    box-shadow: 0 6px 18px color-mix(in srgb, black 20%, transparent);
}

.galaxy-name-banner-logo img {
    width: 100%;
    height: 100%;
    object-fit: contain;
}

.galaxy-name-close {
    position: relative;
    display: inline-flex;
    width: 2rem;
    height: 2rem;
    flex-shrink: 0;
    align-self: flex-start;
    align-items: center;
    justify-content: center;
    padding: 0;
    border: 0;
    border-radius: 9999px;
    background: color-mix(in srgb, white 18%, transparent);
    color: white;
    cursor: pointer;
}

.galaxy-name-close:hover:not(:disabled) {
    background: color-mix(in srgb, white 30%, transparent);
}

.galaxy-name-close:focus-visible {
    outline: 2px solid white;
    outline-offset: 2px;
}

.galaxy-name-close :deep(svg) {
    width: 1.1rem;
    height: 1.1rem;
}

.galaxy-name-avatar-column {
    display: flex;
    width: 5rem;
    flex-shrink: 0;
    flex-direction: column;
    align-items: center;
    gap: 0.35rem;
    margin-top: 1.6rem;
}

.galaxy-name-skin-caption {
    max-width: 100%;
    color: var(--color-text-tertiary);
    font-size: 0.6875rem;
    font-weight: 600;
    line-height: 1.3;
    text-align: center;
    overflow-wrap: anywhere;
    cursor: help;
}

.galaxy-name-avatar {
    display: flex;
    width: 5rem;
    height: 5rem;
    flex-shrink: 0;
    align-items: center;
    justify-content: center;
    padding: 0.5rem;
    border: 1px solid var(--surface-5);
    border-radius: 1rem;
    background: var(--surface-2);
}

.galaxy-name-avatar img {
    width: 100%;
    height: 100%;
    border-radius: 0.4rem;
    image-rendering: pixelated;
}

.galaxy-name-field {
    position: relative;
    display: flex;
    align-items: center;
}

.galaxy-name-field input {
    width: 100%;
    height: 3.25rem;
    padding: 0 4rem 0 1rem;
    border: 2px solid var(--surface-5);
    border-radius: 0.85rem;
    background: var(--surface-4);
    color: var(--color-text-primary);
    font-size: 1.25rem;
    font-weight: 700;
    letter-spacing: 0.01em;
    outline: none;
    transition:
        border-color 150ms ease,
        box-shadow 150ms ease;
}

.galaxy-name-field input::placeholder {
    color: var(--color-text-tertiary);
    font-weight: 500;
    opacity: 0.7;
}

.galaxy-name-field input:focus {
    border-color: var(--color-brand);
    box-shadow: 0 0 0 4px var(--color-brand-highlight);
}

.galaxy-name-field.is-invalid input {
    border-color: var(--color-red);
}

.galaxy-name-field.is-invalid input:focus {
    box-shadow: 0 0 0 4px color-mix(in srgb, var(--color-red) 18%, transparent);
}

.galaxy-name-field input:disabled {
    opacity: 0.6;
}

.galaxy-name-counter {
    position: absolute;
    right: 0.9rem;
    color: var(--color-text-tertiary);
    font-size: 0.8125rem;
    font-variant-numeric: tabular-nums;
    font-weight: 600;
    pointer-events: none;
}

.galaxy-name-counter.is-full {
    color: var(--color-orange, #b45309);
}

/* Looks like an in-game name tag. */
.galaxy-nametag {
    min-width: 0;
    padding: 0.1rem 0.5rem;
    border-radius: 0.25rem;
    background: rgba(0, 0, 0, 0.62);
    color: white;
    font-size: 0.8125rem;
    font-weight: 600;
}

.galaxy-name-rules {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    margin: 0;
    padding: 0.75rem 0.9rem;
    border: 1px solid var(--surface-5);
    border-radius: 0.85rem;
    background: var(--surface-4);
    list-style: none;
    font-size: 0.8125rem;
}

.galaxy-name-rules li {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--color-text-tertiary);
}

.galaxy-name-rules li :deep(svg) {
    width: 1rem;
    height: 1rem;
    flex-shrink: 0;
    padding: 0.1rem;
    border-radius: 9999px;
    background: var(--surface-2);
}

.galaxy-rule-dot {
    display: inline-flex;
    width: 1rem;
    height: 1rem;
    flex-shrink: 0;
    align-items: center;
    justify-content: center;
}

.galaxy-rule-dot::before {
    width: 0.4rem;
    height: 0.4rem;
    border-radius: 9999px;
    background: var(--surface-5);
    content: '';
}

.galaxy-name-rules li.is-ok,
.galaxy-name-rules li.is-saved {
    color: var(--color-text-primary);
}

.galaxy-name-rules li.is-ok :deep(svg),
.galaxy-name-rules li.is-saved :deep(svg) {
    background: var(--color-brand-highlight);
    color: var(--color-brand);
}

.galaxy-name-rules li.is-saved {
    color: var(--color-brand);
    font-weight: 600;
}

.galaxy-name-rules li.is-fail {
    color: var(--color-red);
    font-weight: 600;
}

.galaxy-name-rules li.is-fail :deep(svg) {
    background: color-mix(in srgb, var(--color-red) 14%, transparent);
    color: var(--color-red);
}

.galaxy-name-rules li.is-warn {
    color: var(--color-orange, #b45309);
    font-weight: 600;
}

.galaxy-name-rules li.is-warn :deep(svg) {
    background: transparent;
}

.galaxy-name-chip {
    display: inline-flex;
    height: 2rem;
    align-items: center;
    gap: 0.4rem;
    padding: 0 0.75rem 0 0.3rem;
    border: 1px solid var(--surface-5);
    border-radius: 9999px;
    background: var(--surface-4);
    color: var(--color-text-primary);
    font-size: 0.8125rem;
    font-weight: 600;
    cursor: pointer;
}

.galaxy-name-chip img {
    width: 1.4rem;
    height: 1.4rem;
    border-radius: 0.3rem;
    image-rendering: pixelated;
}

.galaxy-name-chip:hover:not(:disabled) {
    border-color: var(--color-brand);
}

.galaxy-name-chip.is-selected {
    border-color: var(--color-brand);
    background: var(--color-brand-highlight);
    color: var(--color-brand);
}

.galaxy-name-chip:focus-visible {
    outline: 2px solid var(--color-brand);
    outline-offset: 2px;
}

.galaxy-name-explain {
    display: flex;
    gap: 0.6rem;
    padding: 0.75rem 0.9rem;
    border-radius: 0.85rem;
    background: var(--surface-2);
    color: var(--color-text-default);
    font-size: 0.8125rem;
    line-height: 1.5;
}

.galaxy-name-explain :deep(svg) {
    width: 1.1rem;
    height: 1.1rem;
    flex-shrink: 0;
    margin-top: 0.1rem;
    color: var(--color-brand);
}

.galaxy-name-error {
    margin: 0;
    padding: 0.6rem 0.8rem;
    border-radius: 0.75rem;
    background: color-mix(in srgb, var(--color-red) 10%, transparent);
    color: var(--color-red);
    font-size: 0.8125rem;
    font-weight: 600;
}
</style>
