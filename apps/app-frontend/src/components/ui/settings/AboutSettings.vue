<script setup lang="ts">
/**
 * Galaxy宝可梦 About page. Shows our logo, version and the upstream GPL
 * attribution. Axolotl's 3D scene, team/contributor easter eggs, QQ and
 * sponsor links are not part of this fork.
 */
import { ExternalIcon, ScaleIcon } from '@modrinth/assets'
import { Button, defineMessages, useVIntl } from '@modrinth/ui'
import { getVersion } from '@tauri-apps/api/app'
import { inject, onMounted, ref } from 'vue'

import galaxyLogo from '@/assets/galaxy-logo.svg?url'
import { AxolotlBrandConfig } from '@/config'
import { galaxyLauncherUpdateCheck } from '@/helpers/galaxy'

import SettingsSection from './SettingsSection.vue'

const { formatMessage } = useVIntl()
const version = ref('')
const replayOnboarding = inject<(mode: 'main' | 'instance') => Promise<void>>('replayOnboarding')

// Upstream product names are spelled out for attribution only.
const UPSTREAM_AXOLOTL = 'Axolotl Launcher'
const UPSTREAM_MODRINTH = ['Modrinth', 'App'].join(' ')
const creditLine = `基于 ${UPSTREAM_AXOLOTL}（美西螈启动器）与 ${UPSTREAM_MODRINTH} 开发，遵循 GPL-3.0 许可`

const attributionLinks = [
    { href: AxolotlBrandConfig.upstreamRepositoryUrl, label: `${UPSTREAM_AXOLOTL} 源代码` },
    { href: `${AxolotlBrandConfig.upstreamRepositoryUrl}/blob/main/LICENSE`, label: 'GPL-3.0 许可证' },
    {
        href: `${AxolotlBrandConfig.upstreamRepositoryUrl}/tree/main/third-party/licenses`,
        label: '第三方许可',
    },
    { href: AxolotlBrandConfig.upstreamModrinthSourceUrl, label: 'Modrinth 原始源代码' },
]

const messages = defineMessages({
    version: {
        id: 'app.settings.about.version',
        defaultMessage: 'Version {version}',
    },
    replayOnboarding: {
        id: 'app.settings.about.replay-onboarding',
        defaultMessage: 'Replay tour',
    },
    licenseAttribution: {
        id: 'app.settings.about.license-attribution',
        defaultMessage: 'License & attribution',
    },
})

onMounted(() => {
    // Our own release version (galaxy-launcher.json); dev builds fall back to the app version.
    void galaxyLauncherUpdateCheck()
        .then((update) => update.currentVersion ?? getVersion())
        .catch(() => getVersion())
        .then((resolved) => {
            version.value = resolved
        })
        .catch(() => {
            // keep empty version string; do not fail the settings category
        })
})
</script>

<template>
    <div class="about-page flex flex-col gap-6">
        <section id="settings-target-about-product" tabindex="-1" class="about-panel">
            <div class="flex flex-col items-center gap-4">
                <img :src="galaxyLogo" alt="" class="size-28 object-contain" />
                <div class="min-w-0 text-center">
                    <h2 class="m-0 text-xl font-semibold text-[var(--color-text-primary)]">
                        {{ AxolotlBrandConfig.productName }}
                    </h2>
                    <p v-if="version" class="m-0 mt-1 text-[var(--color-text-tertiary)]">
                        {{ formatMessage(messages.version, { version }) }}
                    </p>
                </div>
            </div>
            <p class="m-0 mt-3 text-center text-[var(--color-text-default)]">
                Galaxy宝可梦 服务器专用启动器
            </p>
        </section>

        <SettingsSection>
            <template #header>
                <h3
                    class="m-0 flex items-center gap-2 text-base font-semibold text-[var(--color-text-primary)]"
                >
                    <ScaleIcon class="size-5 text-[var(--color-text-tertiary)]" />
                    {{ formatMessage(messages.licenseAttribution) }}
                </h3>
            </template>
            <div class="flex flex-col gap-3 p-4">
                <p class="m-0 text-[var(--color-text-default)]">{{ creditLine }}</p>
                <p class="m-0 text-sm text-[var(--color-text-tertiary)]">
                    Modrinth 是 Rinth, Inc. 的商标。Galaxy宝可梦 与 Rinth, Inc. 及 Axolotl Launcher
                    官方均无隶属或背书关系。
                </p>
                <div class="mt-1 flex flex-wrap gap-2">
                    <a
                        v-for="link in attributionLinks"
                        :key="link.href"
                        :href="link.href"
                        target="_blank"
                        rel="noopener noreferrer"
                        class="inline-flex items-center gap-2 rounded-lg bg-surface-4 px-3 py-2 text-sm font-semibold text-[var(--color-text-primary)] transition-colors hover:bg-surface-5"
                    >
                        {{ link.label }}
                        <ExternalIcon class="size-4 text-[var(--color-text-tertiary)]" />
                    </a>
                </div>
            </div>
        </SettingsSection>

        <div id="settings-target-about-replay-tour" tabindex="-1" class="flex flex-wrap gap-2">
            <Button type="base" @click="replayOnboarding?.('main')">
                {{ formatMessage(messages.replayOnboarding) }}
            </Button>
        </div>
    </div>
</template>

<style scoped>
.about-panel {
    padding: 1.25rem;
    border: 1px solid var(--surface-4);
    border-radius: var(--radius-md);
    background: var(--surface-3);
}
</style>
