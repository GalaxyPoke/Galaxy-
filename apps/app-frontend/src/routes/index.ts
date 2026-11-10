import './meta'

import { createPinia, getActivePinia, setActivePinia } from 'pinia'
import { createRouter, createWebHistory } from 'vue-router'

import { discard_instance_upgrade_plan } from '@/helpers/instance-upgrade'
import { clearUpgradeFlow, peekUpgradeFlow } from '@/store/navigation-return'

import { homeRoutes } from './home'
import { instanceRoutes } from './instance'
import { utilityRoutes } from './utility'

/**
 * Application router. Domain tables live in sibling modules; URLs and most
 * names match the legacy `routes.js` table (names are PascalCase).
 *
 * Galaxy宝可梦: only Home, Settings, Skins and the fixed instance page are routed.
 * Discover / Multiplayer / Lab / Library / headless-demo route tables are kept
 * on disk for upstream merges but are intentionally not registered.
 */
export default createRouter({
    history: createWebHistory(),
    routes: [
        ...homeRoutes,
        ...utilityRoutes,
        ...instanceRoutes,
        // Any removed page (browse, library, worlds, ...) falls back to home.
        { path: '/:pathMatch(.*)*', redirect: '/' },
    ],
    linkActiveClass: 'router-link-active',
    linkExactActiveClass: 'router-link-exact-active',
    beforeEach() {
        // Use store helpers that bootstrap pinia if missing. Throwing here would
        // abort navigation and leave the user stuck on the current page.
        try {
            if (!getActivePinia()) {
                setActivePinia(createPinia())
            }
            const parkedUpgrade = peekUpgradeFlow()
            if (parkedUpgrade) {
                if (parkedUpgrade.plan?.id)
                    void discard_instance_upgrade_plan(parkedUpgrade.plan.id).catch(console.error)
                clearUpgradeFlow()
            }
        } catch (error) {
            console.warn('[router] navigation-return guard failed; allowing navigation', error)
        }
    },
    scrollBehavior(to, from) {
        if (to.path === from.path) return
        // Sometimes Vue's scroll behavior is not working as expected, so we need to manually scroll to top (especially on Linux)
        document.querySelector('.app-viewport')?.scrollTo(0, 0)
        return {
            el: '.app-viewport',
            top: 0,
        }
    },
})
