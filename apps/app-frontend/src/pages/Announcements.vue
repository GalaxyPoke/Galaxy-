<script setup lang="ts">
/**
 * Galaxy宝可梦 /announcements: every active announcement with category filters.
 * Data: useGalaxyAnnouncements (remote slot → cache → bundled, see
 * galaxy/announcement-source.ts). Titles and lines are rendered through
 * GalaxyMinecraftText (colour codes, no HTML).
 */
import {
    ArrowLeftIcon,
    ChevronDownIcon,
    ChevronUpIcon,
    ClockIcon,
    InfoIcon,
    TerminalSquareIcon,
} from '@modrinth/assets'
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import GalaxyMinecraftText from '@/components/ui/GalaxyMinecraftText.vue'
import {
    formatGalaxyDateTime,
    type GalaxyAnnouncement,
    galaxyAnnouncementButtonLabel,
    galaxyAnnouncementCommand,
} from '@/galaxy/announcements'
import { useGalaxyAnnouncements } from '@/galaxy/use-announcements'

const TEXT = {
    back: '返回主页',
    title: '服务器公告',
    count: '共 {count} 条',
    all: '全部',
    pinned: '置顶',
    mustRead: '必读',
    published: '发布于 {date}',
    until: '活动至 {date}',
    expand: '展开全文',
    collapse: '收起',
    commandPrefix: '进服后输入',
    emptyAll: '暂时没有公告',
    emptyCategory: '「{category}」分类下暂时没有公告',
    showAll: '查看全部公告',
    sourceBundled: '公告来自启动器内置内容',
    sourceRemote: '公告来自服务器',
    sourceCache: '暂时无法连接服务器，显示的是上次获取的公告',
}

const COLLAPSED_LINES = 4

const route = useRoute()
const router = useRouter()
const { announcements, source, now } = useGalaxyAnnouncements()

function fill(template: string, values: Record<string, string | number>) {
    return template.replace(/\{(\w+)\}/g, (_, key) => String(values[key] ?? ''))
}

const categories = computed(() => {
    const counts = new Map<string, number>()
    for (const item of announcements.value)
        counts.set(item.category, (counts.get(item.category) ?? 0) + 1)
    return [...counts.entries()].map(([name, count]) => ({ name, count }))
})

const selectedCategory = computed(() => {
    const value = route.query.category
    return typeof value === 'string' && value ? value : null
})

const filtered = computed(() =>
    selectedCategory.value
        ? announcements.value.filter((item) => item.category === selectedCategory.value)
        : announcements.value,
)

function selectCategory(category: string | null) {
    void router.replace({ query: category ? { category } : {} })
}

const expanded = ref(new Set<string>())
const highlighted = ref<string | null>(null)

function isLong(item: GalaxyAnnouncement) {
    return item.content.length > COLLAPSED_LINES
}

function isExpanded(item: GalaxyAnnouncement) {
    return expanded.value.has(item.id)
}

function visibleLines(item: GalaxyAnnouncement) {
    return isLong(item) && !isExpanded(item) ? item.content.slice(0, COLLAPSED_LINES) : item.content
}

function toggle(item: GalaxyAnnouncement) {
    const next = new Set(expanded.value)
    if (next.has(item.id)) next.delete(item.id)
    else next.add(item.id)
    expanded.value = next
}

const sourceNote = computed(() => {
    if (source.value === 'remote') return TEXT.sourceRemote
    if (source.value === 'cache') return TEXT.sourceCache
    return TEXT.sourceBundled
})

/** Home links to `/announcements#announcement-<id>`: open and scroll to that card. */
async function focusHash() {
    const match = /^#announcement-(.+)$/.exec(route.hash)
    if (!match) return
    const id = decodeURIComponent(match[1])
    if (!announcements.value.some((item) => item.id === id)) return
    expanded.value = new Set([...expanded.value, id])
    highlighted.value = id
    await nextTick()
    document.getElementById(`announcement-${id}`)?.scrollIntoView({ block: 'start' })
    setTimeout(() => {
        if (highlighted.value === id) highlighted.value = null
    }, 2000)
}

onMounted(() => void focusHash())
watch(
    () => route.hash,
    () => void focusHash(),
)
</script>

<template>
    <div class="galaxy-announcements mx-auto flex w-full max-w-[60rem] flex-col gap-5 px-6 pb-8 pt-5">
        <router-link to="/" class="galaxy-back">
            <ArrowLeftIcon aria-hidden="true" />
            {{ TEXT.back }}
        </router-link>

        <header class="flex flex-col gap-3">
            <div class="flex items-baseline gap-3">
                <h1 class="m-0 text-3xl font-extrabold text-[var(--color-text-primary)]">
                    {{ TEXT.title }}
                </h1>
                <span class="text-sm font-medium text-[var(--color-text-tertiary)]">{{
                    fill(TEXT.count, { count: announcements.length })
                }}</span>
            </div>
            <div class="flex flex-wrap gap-2" role="toolbar" :aria-label="TEXT.title">
                <button
                    type="button"
                    class="galaxy-filter"
                    :class="{ 'is-active': !selectedCategory }"
                    :aria-pressed="!selectedCategory"
                    @click="selectCategory(null)"
                >
                    {{ TEXT.all }}
                    <span class="galaxy-filter-count">{{ announcements.length }}</span>
                </button>
                <button
                    v-for="category in categories"
                    :key="category.name"
                    type="button"
                    class="galaxy-filter"
                    :class="{ 'is-active': selectedCategory === category.name }"
                    :aria-pressed="selectedCategory === category.name"
                    @click="selectCategory(category.name)"
                >
                    {{ category.name }}
                    <span class="galaxy-filter-count">{{ category.count }}</span>
                </button>
            </div>
        </header>

        <section class="flex flex-col gap-3">
            <article
                v-for="item in filtered"
                :id="`announcement-${item.id}`"
                :key="item.id"
                class="galaxy-notice"
                :class="{ 'is-highlighted': highlighted === item.id, 'is-pinned': item.pinned }"
            >
                <div class="flex flex-wrap items-center gap-1.5">
                    <span class="galaxy-badge">{{ item.category }}</span>
                    <span v-if="item.pinned" class="galaxy-badge is-pinned">{{ TEXT.pinned }}</span>
                    <span v-if="item.mustRead" class="galaxy-badge is-must-read">{{
                        TEXT.mustRead
                    }}</span>
                    <span v-if="item.endAt" class="galaxy-badge is-until">
                        <ClockIcon aria-hidden="true" />
                        {{ fill(TEXT.until, { date: formatGalaxyDateTime(item.endAt, now) }) }}
                    </span>
                    <time
                        class="ml-auto text-xs text-[var(--color-text-tertiary)]"
                        :datetime="item.createdAt"
                        >{{
                            fill(TEXT.published, { date: formatGalaxyDateTime(item.createdAt, now) })
                        }}</time
                    >
                </div>
                <h2 class="m-0 text-lg font-bold leading-snug text-[var(--color-text-primary)]">
                    <GalaxyMinecraftText :text="item.title" />
                </h2>
                <div
                    class="galaxy-notice-body"
                    :class="{ 'is-collapsed': isLong(item) && !isExpanded(item) }"
                >
                    <p
                        v-for="(line, index) in visibleLines(item)"
                        :key="index"
                        class="m-0 text-sm leading-6 text-[var(--color-text-default)]"
                    >
                        <GalaxyMinecraftText :text="line" />
                    </p>
                </div>
                <div
                    v-if="isLong(item) || galaxyAnnouncementCommand(item)"
                    class="flex flex-wrap items-center gap-2"
                >
                    <span v-if="galaxyAnnouncementCommand(item)" class="galaxy-command">
                        <TerminalSquareIcon aria-hidden="true" />
                        <span v-if="galaxyAnnouncementButtonLabel(item)" class="font-semibold"
                            >{{ galaxyAnnouncementButtonLabel(item) }}：</span
                        >{{ TEXT.commandPrefix }}
                        <code>{{ galaxyAnnouncementCommand(item) }}</code>
                    </span>
                    <button
                        v-if="isLong(item)"
                        type="button"
                        class="galaxy-toggle ml-auto"
                        :aria-expanded="isExpanded(item)"
                        @click="toggle(item)"
                    >
                        {{ isExpanded(item) ? TEXT.collapse : TEXT.expand }}
                        <ChevronUpIcon v-if="isExpanded(item)" aria-hidden="true" />
                        <ChevronDownIcon v-else aria-hidden="true" />
                    </button>
                </div>
            </article>

            <div v-if="!filtered.length" class="galaxy-empty">
                <InfoIcon aria-hidden="true" />
                <span>{{
                    selectedCategory
                        ? fill(TEXT.emptyCategory, { category: selectedCategory })
                        : TEXT.emptyAll
                }}</span>
                <button
                    v-if="selectedCategory"
                    type="button"
                    class="galaxy-toggle"
                    @click="selectCategory(null)"
                >
                    {{ TEXT.showAll }}
                </button>
            </div>
        </section>

        <p class="galaxy-source m-0" :class="{ 'is-cache': source === 'cache' }">
            <InfoIcon aria-hidden="true" />
            {{ sourceNote }}
        </p>
    </div>
</template>

<style scoped>
.galaxy-back {
    display: inline-flex;
    align-self: flex-start;
    align-items: center;
    gap: 0.35rem;
    padding: 0.3rem 0.7rem 0.3rem 0.5rem;
    border-radius: 9999px;
    color: var(--color-text-default);
    font-size: 0.875rem;
    font-weight: 600;
    text-decoration: none;
}

.galaxy-back:hover {
    background: var(--surface-4);
    color: var(--color-text-primary);
}

.galaxy-back :deep(svg) {
    width: 1rem;
    height: 1rem;
}

.galaxy-filter {
    display: inline-flex;
    height: 2.1rem;
    align-items: center;
    gap: 0.45rem;
    padding: 0 0.6rem 0 0.9rem;
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

.galaxy-filter:hover:not(.is-active) {
    border-color: color-mix(in srgb, var(--color-brand) 45%, var(--surface-5));
}

.galaxy-filter.is-active {
    border-color: transparent;
    background: var(--color-brand-fill, var(--color-brand));
    color: var(--color-brand-fill-contrast, var(--color-accent-contrast));
}

.galaxy-filter-count {
    min-width: 1.4rem;
    padding: 0.05rem 0.4rem;
    border-radius: 9999px;
    background: var(--surface-2);
    color: var(--color-text-tertiary);
    font-size: 0.75rem;
    text-align: center;
}

.galaxy-filter.is-active .galaxy-filter-count {
    background: color-mix(in srgb, black 12%, transparent);
    color: inherit;
}

.galaxy-filter:focus-visible,
.galaxy-back:focus-visible,
.galaxy-toggle:focus-visible {
    outline: 2px solid var(--color-brand);
    outline-offset: 2px;
}

.galaxy-notice {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    padding: 1rem 1.25rem 1.1rem;
    border: 1px solid var(--surface-5);
    border-radius: 1rem;
    background: var(--surface-4);
    box-shadow: var(--shadow-card);
    scroll-margin-top: 1rem;
    transition:
        border-color 300ms ease,
        box-shadow 300ms ease;
}

.galaxy-notice.is-pinned {
    border-left: 4px solid var(--color-brand);
    padding-left: calc(1.25rem - 3px);
}

.galaxy-notice.is-highlighted {
    border-color: var(--color-brand);
    box-shadow:
        0 0 0 3px var(--color-brand-highlight),
        var(--shadow-card);
}

.galaxy-notice-body {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
}

.galaxy-notice-body.is-collapsed {
    mask-image: linear-gradient(to bottom, black 60%, transparent 100%);
    -webkit-mask-image: linear-gradient(to bottom, black 60%, transparent 100%);
}

.galaxy-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.1rem 0.5rem;
    border: 1px solid var(--surface-5);
    border-radius: 9999px;
    background: var(--surface-2);
    color: var(--color-text-default);
    font-size: 0.75rem;
    font-weight: 600;
    white-space: nowrap;
}

.galaxy-badge :deep(svg) {
    width: 0.8rem;
    height: 0.8rem;
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

.galaxy-badge.is-until {
    border-color: color-mix(in srgb, #d97706 35%, transparent);
    background: color-mix(in srgb, #f59e0b 12%, transparent);
    color: #b45309;
}

.galaxy-command {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.3rem 0.7rem;
    border-radius: 0.6rem;
    background: var(--surface-2);
    color: var(--color-text-default);
    font-size: 0.8125rem;
}

.galaxy-command :deep(svg) {
    width: 0.95rem;
    height: 0.95rem;
    color: var(--color-brand);
}

.galaxy-command code {
    padding: 0.05rem 0.35rem;
    border-radius: 0.35rem;
    background: var(--surface-4);
    color: var(--color-text-primary);
    font-size: 0.8rem;
    font-weight: 600;
}

.galaxy-toggle {
    display: inline-flex;
    align-items: center;
    gap: 0.2rem;
    padding: 0.2rem 0.4rem;
    border: 0;
    border-radius: 0.5rem;
    background: transparent;
    color: var(--color-brand);
    font-size: 0.8125rem;
    font-weight: 600;
    cursor: pointer;
}

.galaxy-toggle:hover {
    background: var(--color-brand-highlight);
}

.galaxy-toggle :deep(svg) {
    width: 1rem;
    height: 1rem;
}

.galaxy-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.5rem;
    padding: 2.5rem 1rem;
    border: 1px dashed var(--surface-5);
    border-radius: 1rem;
    background: var(--surface-4);
    color: var(--color-text-tertiary);
    font-size: 0.875rem;
}

.galaxy-empty :deep(svg) {
    width: 1.5rem;
    height: 1.5rem;
}

.galaxy-source {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.35rem;
    color: var(--color-text-tertiary);
    font-size: 0.75rem;
}

.galaxy-source.is-cache {
    color: #b45309;
}

.galaxy-source :deep(svg) {
    width: 0.9rem;
    height: 0.9rem;
}
</style>
