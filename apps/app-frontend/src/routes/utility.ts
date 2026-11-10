import './meta'

import type { RouteRecordRaw } from 'vue-router'

/**
 * Top-level utility pages that do not belong to a larger content vertical.
 * Galaxy宝可梦: /worlds, /create, /downloads, /screenshots and /help/drop were removed.
 */
export const utilityRoutes: RouteRecordRaw[] = [
    {
        path: '/settings',
        name: 'Settings',
        component: () => import('@/pages/Settings.vue'),
        meta: {
            breadcrumb: [{ name: 'Settings' }],
            discordActivity: 'Idling...',
        },
    },
    {
        path: '/skins',
        name: 'SkinSelector',
        component: () => import('@/pages/Skins.vue'),
        meta: {
            breadcrumb: [{ name: 'Skin selector' }],
            discordActivity: 'Changing skins...',
        },
    },
]
