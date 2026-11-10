<template>
    <div
        v-if="accounts.length === 0"
        class="flex flex-col gap-3 bg-surface-4 border border-solid border-surface-5 rounded-xl p-3 mt-2"
    >
        <span>{{ formatMessage(messages.notSignedIn) }}</span>
        <Button
            type="colored"
            color="brand"
            data-onboarding-id="offline-account-entry"
            :disabled="loginDisabled"
            @click="showPlayerNameModal()"
            ><PlusIcon />
            {{ GALAXY_TEXT.setName }}
        </Button>
    </div>
    <!--
        Galaxy宝可梦: in the header player popout (`expanded`) the chip already shows the
        selected name, so the card drops the accordion header and renders only the list.
    -->
    <component
        :is="expanded ? 'div' : Accordion"
        v-else
        v-bind="
            expanded
                ? { class: 'w-full' }
                : {
                      class: 'w-full mt-2 bg-surface-4 border border-solid border-surface-5 rounded-xl overflow-clip',
                      buttonClass: 'button-base w-full bg-transparent px-3 py-2 border-0 cursor-pointer',
                  }
        "
    >
        <template v-if="!expanded" #title>
            <div class="flex gap-2 w-full min-w-0">
                <Avatar
                    size="36px"
                    :src="selectedAccount ? avatarUrl : galaxyLogo"
                    :pixelated="Boolean(selectedAccount)"
                    :unframed-natural-width="72"
                />
                <div class="flex flex-col items-start w-full min-w-0">
                    <span class="truncate w-full text-left">{{
                        selectedAccount
                            ? selectedAccount.profile.name
                            : formatMessage(messages.selectAccount)
                    }}</span>
                    <span class="text-[var(--color-text-tertiary)] text-xs">
                        {{ formatMessage(messages.offlineAccount) }}
                    </span>
                </div>
            </div>
        </template>
        <div
            :class="
                expanded
                    ? 'pb-1'
                    : 'bg-surface-4 pt-1 pb-2 border-0 border-t border-solid border-surface-5'
            "
        >
            <div
                v-for="account in accounts"
                :key="account.account_id"
                class="flex gap-1 items-center"
            >
                <button
                    class="flex items-center flex-shrink flex-grow overflow-clip gap-2 p-2 border-0 bg-transparent cursor-pointer button-base min-w-0"
                    @click="setAccount(account)"
                >
                    <RadioButtonCheckedIcon
                        v-if="selectedAccount && selectedAccount.account_id === account.account_id"
                        class="w-5 h-5 text-brand shrink-0"
                    />
                    <RadioButtonIcon
                        v-else
                        class="w-5 h-5 text-[var(--color-text-tertiary)] shrink-0"
                    />
                    <Avatar
                        :src="getAccountAvatarUrl(account)"
                        size="24px"
                        pixelated
                        :unframed-natural-width="72"
                    />
                    <div class="flex flex-1 min-w-0 flex-col text-left">
                        <p
                            class="m-0 truncate text-left"
                            :class="
                                selectedAccount && selectedAccount.account_id === account.account_id
                                    ? 'text-[var(--color-text-primary)] font-semibold'
                                    : 'text-[var(--color-text-default)]'
                            "
                        >
                            {{ account.profile.name }}
                        </p>
                    </div>
                    <span class="text-[var(--color-text-tertiary)] text-xs shrink-0">
                        {{ formatMessage(messages.offlineBadge) }}
                    </span>
                </button>
                <div class="flex shrink-0 items-center">
                    <button
                        v-tooltip="formatMessage(messages.copyUuid)"
                        type="button"
                        class="button-base border-0 bg-transparent p-1.5 cursor-pointer text-[var(--color-text-tertiary)] hover:text-brand"
                        @click="copyAccountUuid(account)"
                    >
                        <CopyIcon />
                    </button>
                    <button
                        v-tooltip="formatMessage(messages.removeAccount)"
                        type="button"
                        class="button-base border-0 bg-transparent p-1.5 cursor-pointer text-[var(--color-text-tertiary)] hover:text-red"
                        @click="logout(account)"
                    >
                        <TrashIcon />
                    </button>
                </div>
            </div>
            <div class="flex flex-col gap-2 px-2 pt-2">
                <Button
                    class="w-full"
                    data-onboarding-id="offline-account-entry"
                    :disabled="loginDisabled"
                    @click="showPlayerNameModal()"
                    ><PlusIcon />
                    {{ GALAXY_TEXT.addName }}
                </Button>
            </div>
        </div>
    </component>
    <GalaxyPlayerNameModal
        ref="playerNameModal"
        variant="add"
        :disabled="loginDisabled"
        @busy="setLoginDisabled"
        @saved="onPlayerNameSaved"
    />
</template>

<script setup lang="ts">
/**
 * Galaxy宝可梦: offline-only account card. The server runs with
 * `online-mode=false`, so players only pick a name (3–16 letters, digits or
 * underscores). The backend `galaxy_set_player` command creates or reuses the
 * offline account (UUID = nameUUIDFromBytes("OfflinePlayer:" + name)) and makes
 * it the default. Microsoft and Yggdrasil sign-in are intentionally not offered.
 */
import {
    CopyIcon,
    PlusIcon,
    RadioButtonCheckedIcon,
    RadioButtonIcon,
    TrashIcon,
} from '@modrinth/assets'
import {
    Accordion,
    Avatar,
    Button,
    defineMessages,
    injectNotificationManager,
    useVIntl,
} from '@modrinth/ui'
import type { Ref } from 'vue'
import { computed, onMounted, onUnmounted, ref } from 'vue'

import galaxyLogo from '@/assets/galaxy-logo.svg?url'
import GalaxyPlayerNameModal from '@/components/ui/GalaxyPlayerNameModal.vue'
import { findVanillaDefaultSkin, getPlayerHeadDataUrl } from '@/galaxy/player-head'
import { defaultSteveHeadUrl } from '@/galaxy/skin-head'
import { compareMinecraftAccounts } from '@/helpers/accounts'
import { get_default_user, remove_user, set_default_user, users } from '@/helpers/auth'
import { process_listener } from '@/helpers/events'
import { galaxyErrorText } from '@/helpers/galaxy'
import type { Skin } from '@/helpers/skins'
import { get_available_skins } from '@/helpers/skins'

const GALAXY_TEXT = {
    setName: '设置游戏名',
    addName: '添加其他游戏名',
    uuidCopyFailed: '复制 UUID 失败',
}

const { formatMessage } = useVIntl()
const { handleError, addNotification } = injectNotificationManager()

withDefaults(
    defineProps<{
        /** Start with the account list open (the header player popout). */
        expanded?: boolean
    }>(),
    { expanded: false },
)

const emit = defineEmits<{
    change: []
    /** The name dialog is opening; the header popout closes so it does not cover it. */
    'name-dialog-open': []
}>()

type MinecraftCredential = {
    account_id: string
    account_type: 'microsoft' | 'offline' | 'yggdrasil'
    profile: {
        id: string
        name: string
    }
}

const accounts: Ref<MinecraftCredential[]> = ref([])
const loginDisabled = ref(false)
const defaultUser = ref<string | undefined>()
const equippedSkin = ref<Skin | null>(null)
const equippedHeadUrl = ref<string | null>(null)
/** In-game default head of every saved name, keyed by name (the selected one uses its equipped skin). */
const accountHeads = ref<Record<string, string>>({})
const accountChangeRevision = ref(0)
let refreshGeneration = 0

const playerNameModal = ref<InstanceType<typeof GalaxyPlayerNameModal> | null>(null)

async function refreshValues() {
    const generation = ++refreshGeneration
    // Offline accounts never need the Microsoft/Mojang services, so always ask
    // the backend in offline mode.
    const [selectedUser, userList] = await Promise.all([
        get_default_user(true).catch(handleError),
        users(true, true).catch(handleError),
    ])
    if (generation !== refreshGeneration) return
    if (Array.isArray(userList)) {
        accounts.value = (userList as unknown as MinecraftCredential[])
            .filter((account) => account.account_type === 'offline')
            .sort(compareMinecraftAccounts)
    }
    defaultUser.value = selectedUser ?? undefined

    try {
        const skins = await get_available_skins()
        if (generation !== refreshGeneration) return
        void refreshAccountHeads(skins, generation)
        await setEquippedSkin(skins.find((skin) => skin.is_equipped) ?? null)
    } catch {
        equippedSkin.value = null
        equippedHeadUrl.value = null
    }
}

/**
 * Without a skin of its own a name wears the vanilla default skin picked from
 * its offline UUID, so every saved name shows the head other players see.
 */
async function refreshAccountHeads(skins: Skin[], generation: number) {
    const heads: Record<string, string> = {}
    for (const account of accounts.value) {
        const name = account.profile.name
        const skin = findVanillaDefaultSkin(skins, name)
        if (!skin) continue
        try {
            heads[name] = await getPlayerHeadDataUrl(skin)
        } catch (error) {
            console.warn('Failed to render default head for', name, error)
        }
    }
    if (generation === refreshGeneration) accountHeads.value = heads
}

async function setEquippedSkin(skin: Skin | null) {
    equippedSkin.value = skin
    equippedHeadUrl.value = null
    if (!skin) return
    try {
        const headUrl = await getPlayerHeadDataUrl(skin)
        if (equippedSkin.value?.texture_key === skin.texture_key) equippedHeadUrl.value = headUrl
    } catch (error) {
        console.warn('Failed to get head render for equipped skin:', error)
    }
}

function setLoginDisabled(value: boolean) {
    loginDisabled.value = value
}

/** Kept for callers that used to start the Microsoft sign-in: opens the name prompt instead. */
function login() {
    showPlayerNameModal()
}

const selectedAccount = computed(() =>
    accounts.value.find((account) => account.account_id === defaultUser.value),
)

const avatarUrl = computed(
    () =>
        equippedHeadUrl.value ??
        (selectedAccount.value ? accountHeads.value[selectedAccount.value.profile.name] : null) ??
        defaultSteveHeadUrl,
)

/** Read by the header player chip and the home play dock. */
const selectedName = computed(() => selectedAccount.value?.profile.name ?? null)
const hasAccounts = computed(() => accounts.value.length > 0)

defineExpose({
    accountChangeRevision,
    login,
    refreshValues,
    setEquippedSkin,
    setLoginDisabled,
    loginDisabled,
    selectedName,
    avatarUrl,
    hasAccounts,
})

function getAccountAvatarUrl(account: MinecraftCredential) {
    if (account.account_id === selectedAccount.value?.account_id && equippedHeadUrl.value) {
        return equippedHeadUrl.value
    }
    return accountHeads.value[account.profile.name] ?? defaultSteveHeadUrl
}

function notifyAccountChange() {
    accountChangeRevision.value += 1
    emit('change')
}

async function setAccount(account: MinecraftCredential) {
    try {
        await set_default_user(account.account_id)
    } catch (error) {
        handleError(error as Error)
        return
    }
    defaultUser.value = account.account_id
    await refreshValues()
    notifyAccountChange()
}

function showPlayerNameModal() {
    emit('name-dialog-open')
    void playerNameModal.value?.show()
}

/** The dialog already called galaxySetPlayer (the name is now the active account). */
async function onPlayerNameSaved() {
    await refreshValues()
    notifyAccountChange()
}

async function logout(account: MinecraftCredential) {
    await remove_user(account.account_id).catch(handleError)
    await refreshValues()
    if (!selectedAccount.value && accounts.value.length > 0) {
        await setAccount(accounts.value[0])
    } else {
        notifyAccountChange()
    }
}

async function copyAccountUuid(account: MinecraftCredential) {
    try {
        await navigator.clipboard.writeText(account.profile.id)
    } catch (error) {
        addNotification({
            type: 'error',
            title: GALAXY_TEXT.uuidCopyFailed,
            text: galaxyErrorText(error),
        })
    }
}

await refreshValues().catch(console.warn)

onMounted(() => {
    void refreshValues().catch(console.warn)
})

const unlisten = await process_listener(async (e: { event: string }) => {
    if (e.event === 'launched') {
        await refreshValues()
    }
})

onUnmounted(() => {
    refreshGeneration += 1
    unlisten()
})

const messages = defineMessages({
    notSignedIn: {
        id: 'minecraft-account.not-signed-in',
        defaultMessage: 'Not signed in',
    },
    offlineAccount: {
        id: 'minecraft-account.offline-account',
        defaultMessage: 'Offline Minecraft account',
    },
    offlineBadge: {
        id: 'minecraft-account.offline-badge',
        defaultMessage: 'Offline',
    },
    copyUuid: {
        id: 'minecraft-account.copy-uuid',
        defaultMessage: 'Copy UUID',
    },
    removeAccount: {
        id: 'minecraft-account.remove-account',
        defaultMessage: 'Remove account',
    },
    selectAccount: {
        id: 'minecraft-account.select-account',
        defaultMessage: 'Select account',
    },
})
</script>
