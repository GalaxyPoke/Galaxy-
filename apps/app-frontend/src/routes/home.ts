import './meta'

import type { RouteRecordRaw } from 'vue-router'

export const homeRoutes: RouteRecordRaw[] = [
    {
        path: '/',
        name: 'Home',
        component: () => import('@/pages/Index.vue'),
        meta: {
            breadcrumb: [{ name: 'Home' }],
            discordActivity: 'Idling...',
        },
    },
    {
        path: '/announcements',
        name: 'Announcements',
        component: () => import('@/pages/Announcements.vue'),
        meta: {
            breadcrumb: [{ name: 'Announcements' }],
            discordActivity: 'Idling...',
        },
    },
]
