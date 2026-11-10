/**
 * Galaxy宝可梦 新手上路 checklist on the home page: what the launcher cannot
 * detect is remembered in localStorage (first launch, steps ticked by hand,
 * dismissed). "Joined the main server" comes from the data API.
 */
import { reactive, watch } from 'vue'

const STORAGE_KEY = 'galaxy.onboarding.v1'

interface OnboardingState {
    dismissed: boolean
    /** The game was started from this launcher at least once. */
    launched: boolean
    /** Steps ticked by hand when the data API cannot tell. */
    manual: string[]
}

function load(): OnboardingState {
    try {
        const raw = JSON.parse(
            localStorage.getItem(STORAGE_KEY) ?? '{}',
        ) as Partial<OnboardingState>
        return {
            dismissed: raw.dismissed === true,
            launched: raw.launched === true,
            manual: Array.isArray(raw.manual)
                ? raw.manual.filter((id): id is string => typeof id === 'string').slice(0, 10)
                : [],
        }
    } catch {
        return { dismissed: false, launched: false, manual: [] }
    }
}

export const galaxyOnboarding = reactive<OnboardingState>(load())

watch(
    galaxyOnboarding,
    (state) => {
        try {
            localStorage.setItem(STORAGE_KEY, JSON.stringify(state))
        } catch {
            // Storage unavailable: the card just shows again next time.
        }
    },
    { deep: true },
)

export function markGalaxyLaunched() {
    galaxyOnboarding.launched = true
}

export function markGalaxyStepDone(id: string) {
    if (!galaxyOnboarding.manual.includes(id)) galaxyOnboarding.manual.push(id)
}
