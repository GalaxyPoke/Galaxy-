<script setup lang="ts">
/**
 * 新手上路: the first-day checklist on the home page (name → first launch →
 * register on the login server → prologue and starter → main server). Steps
 * the launcher cannot see are ticked by the data API (`known`: the player has
 * joined the main server) or by hand when the API is unavailable.
 */
import { CheckIcon, XIcon } from '@modrinth/assets'
import { computed } from 'vue'

import { galaxyOnboarding, markGalaxyStepDone } from '@/galaxy/onboarding'

const props = defineProps<{
    hasName: boolean
    /** Joined the main server; null when the data API cannot tell. */
    known: boolean | null
    playDisabled?: boolean
}>()

const emit = defineEmits<{
    setName: []
    play: []
}>()

interface Step {
    id: string
    title: string
    text: string
    done: boolean
    action: { label: string; run: () => void; disabled?: boolean } | null
    manual: boolean
}

const steps = computed<Step[]>(() => {
    const joined = props.known === true
    const byHand = (id: string) => galaxyOnboarding.manual.includes(id)
    return [
        {
            id: 'name',
            title: '起一个游戏名',
            text: '游戏名就是你在服务器里的名字，3–16 位英文字母、数字或下划线，起好以后不要随便换。',
            done: props.hasName,
            action: props.hasName ? null : { label: '设置游戏名', run: () => emit('setName') },
            manual: false,
        },
        {
            id: 'launch',
            title: '第一次进入服务器',
            text: '点右下角的「开始游戏」。第一次启动要加载模组和资源，等一两分钟是正常的。',
            done: galaxyOnboarding.launched || joined,
            action:
                galaxyOnboarding.launched || joined
                    ? null
                    : { label: '开始游戏', run: () => emit('play'), disabled: props.playDisabled },
            manual: false,
        },
        {
            id: 'register',
            title: '注册并登录',
            text: '进服后会弹出登录窗口。第一次来按提示注册，以后每次进服输入同一个密码登录。忘记密码请找管理员重置。',
            done: joined || byHand('register'),
            action: null,
            manual: props.known === null,
        },
        {
            id: 'prologue',
            title: '走完序章，领初始宝可梦',
            text: '跟着城门守门人的指引走完《长安旧梦》序章，在精灵司学士那里选初始伙伴，最后到城门祭坛进入正式服。',
            done: joined || byHand('prologue'),
            action: null,
            manual: props.known === null,
        },
    ]
})

const doneCount = computed(() => steps.value.filter((step) => step.done).length)
const visible = computed(() => !galaxyOnboarding.dismissed && doneCount.value < steps.value.length)
/** The first unfinished step is expanded. */
const currentId = computed(() => steps.value.find((step) => !step.done)?.id ?? null)

function dismiss() {
    galaxyOnboarding.dismissed = true
}
</script>

<template>
    <section v-if="visible" class="galaxy-onboarding" aria-labelledby="galaxy-onboarding-title">
        <div class="galaxy-onboarding-head">
            <h2 id="galaxy-onboarding-title">新手上路</h2>
            <span class="galaxy-onboarding-progress">{{ doneCount }}/{{ steps.length }}</span>
            <button type="button" class="galaxy-onboarding-dismiss" @click="dismiss()">
                <XIcon aria-hidden="true" />
                不再显示
            </button>
        </div>
        <ol class="galaxy-onboarding-steps">
            <li
                v-for="(step, index) in steps"
                :key="step.id"
                :class="{ 'is-done': step.done, 'is-current': step.id === currentId }"
            >
                <span class="galaxy-onboarding-mark" aria-hidden="true">
                    <CheckIcon v-if="step.done" />
                    <template v-else>{{ index + 1 }}</template>
                </span>
                <div class="flex min-w-0 flex-1 flex-col gap-1">
                    <span class="galaxy-onboarding-title">
                        {{ step.title }}
                        <span v-if="step.done" class="sr-only">（已完成）</span>
                    </span>
                    <p v-if="step.id === currentId" class="galaxy-onboarding-text">
                        {{ step.text }}
                    </p>
                    <div
                        v-if="step.id === currentId && (step.action || step.manual)"
                        class="flex flex-wrap items-center gap-3"
                    >
                        <button
                            v-if="step.action"
                            type="button"
                            class="galaxy-onboarding-action"
                            :disabled="step.action.disabled"
                            @click="step.action.run()"
                        >
                            {{ step.action.label }}
                        </button>
                        <button
                            v-if="step.manual"
                            type="button"
                            class="galaxy-onboarding-link"
                            @click="markGalaxyStepDone(step.id)"
                        >
                            我已完成这一步
                        </button>
                    </div>
                </div>
            </li>
        </ol>
        <p class="galaxy-onboarding-tip">
            进了正式服以后：按 ESC
            打开菜单，在搜索框输入功能名（家园、任务、通行证……）就能直达；每天记得输入
            <code>/signin</code> 签到。
        </p>
    </section>
</template>

<style scoped>
.galaxy-onboarding {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    padding: 1rem 1.25rem;
    border: 1px solid color-mix(in srgb, var(--color-brand) 35%, var(--surface-5));
    border-radius: 1rem;
    background:
        linear-gradient(
            135deg,
            color-mix(in srgb, var(--color-brand) 9%, transparent),
            transparent 60%
        ),
        var(--surface-4);
    box-shadow: var(--shadow-card);
}

.galaxy-onboarding-head {
    display: flex;
    align-items: center;
    gap: 0.6rem;
}

.galaxy-onboarding-head h2 {
    margin: 0;
    color: var(--color-text-primary);
    font-size: 1.05rem;
    font-weight: 700;
}

.galaxy-onboarding-progress {
    padding: 0.05rem 0.5rem;
    border-radius: 9999px;
    background: var(--color-brand-highlight);
    color: var(--color-brand);
    font-size: 0.75rem;
    font-weight: 700;
}

.galaxy-onboarding-dismiss {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    margin-left: auto;
    padding: 0.25rem 0.5rem;
    border: 0;
    border-radius: 0.5rem;
    background: transparent;
    color: var(--color-text-tertiary);
    font-size: 0.8125rem;
    cursor: pointer;
}

.galaxy-onboarding-dismiss:hover {
    background: var(--surface-2);
    color: var(--color-text-primary);
}

.galaxy-onboarding-dismiss :deep(svg) {
    width: 0.9rem;
    height: 0.9rem;
}

.galaxy-onboarding-steps {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(13rem, 1fr));
    gap: 0.75rem;
    margin: 0;
    padding: 0;
    list-style: none;
}

.galaxy-onboarding-steps li {
    display: flex;
    gap: 0.6rem;
    padding: 0.6rem 0.7rem;
    border: 1px solid var(--surface-5);
    border-radius: 0.75rem;
    background: var(--surface-3, var(--surface-2));
}

.galaxy-onboarding-steps li.is-current {
    border-color: color-mix(in srgb, var(--color-brand) 55%, var(--surface-5));
    background: var(--surface-4);
}

.galaxy-onboarding-mark {
    display: inline-flex;
    width: 1.5rem;
    height: 1.5rem;
    flex-shrink: 0;
    align-items: center;
    justify-content: center;
    border-radius: 9999px;
    background: var(--surface-2);
    color: var(--color-text-tertiary);
    font-size: 0.8rem;
    font-weight: 700;
}

.is-current .galaxy-onboarding-mark {
    background: var(--color-brand);
    color: var(--color-accent-contrast, white);
}

.is-done .galaxy-onboarding-mark {
    background: var(--color-brand-highlight);
    color: var(--color-brand);
}

.galaxy-onboarding-mark :deep(svg) {
    width: 0.9rem;
    height: 0.9rem;
}

.galaxy-onboarding-title {
    color: var(--color-text-primary);
    font-size: 0.875rem;
    font-weight: 700;
}

.is-done .galaxy-onboarding-title {
    color: var(--color-text-tertiary);
    text-decoration: line-through;
}

.galaxy-onboarding-text {
    margin: 0;
    color: var(--color-text-default);
    font-size: 0.8125rem;
    line-height: 1.5;
}

.galaxy-onboarding-action {
    padding: 0.3rem 0.75rem;
    border: 0;
    border-radius: 0.6rem;
    background: var(--color-brand);
    color: var(--color-accent-contrast, white);
    font-size: 0.8125rem;
    font-weight: 700;
    cursor: pointer;
}

.galaxy-onboarding-action:disabled {
    cursor: not-allowed;
    opacity: 0.5;
}

.galaxy-onboarding-link {
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--color-brand);
    font-size: 0.8125rem;
    font-weight: 600;
    cursor: pointer;
}

.galaxy-onboarding-link:hover {
    text-decoration: underline;
}

.galaxy-onboarding-action:focus-visible,
.galaxy-onboarding-link:focus-visible,
.galaxy-onboarding-dismiss:focus-visible {
    outline: 2px solid var(--color-brand);
    outline-offset: 2px;
}

.galaxy-onboarding-tip {
    margin: 0;
    color: var(--color-text-tertiary);
    font-size: 0.8125rem;
}

.galaxy-onboarding-tip code {
    padding: 0 0.3rem;
    border-radius: 0.35rem;
    background: var(--surface-2);
    color: var(--color-text-default);
}
</style>
