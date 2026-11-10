import './meta'

import type { RouteRecordRaw } from 'vue-router'

/**
 * Galaxy宝可梦: the fixed instance page only keeps Content (read-only), Screenshots
 * and Logs. upgrade/*, worlds and files children were removed.
 */
export const instanceRoutes: RouteRecordRaw[] = [
    {
        path: '/instance/:id',
        name: 'Instance',
        component: () => import('@/pages/instance/Index.vue'),
        props: true,
        meta: {
            discordActivity: 'Browsing instances...',
            pageTransitionGroup: 'instance',
        },
        children: [
            {
                path: 'screenshots',
                name: 'InstanceScreenshots',
                component: () => import('@/pages/instance/Screenshots.vue'),
                meta: {
                    useRootContext: true,
                    breadcrumb: [{ name: 'Screenshots' }],
                },
            },
            {
                path: '',
                name: 'Mods',
                component: () => import('@/pages/instance/Mods.vue'),
                meta: {
                    useRootContext: true,
                    breadcrumb: [{ name: 'Content' }],
                },
            },
            {
                path: 'projects/:type',
                name: 'ModsFilter',
                component: () => import('@/pages/instance/Mods.vue'),
                meta: {
                    useRootContext: true,
                    breadcrumb: [{ name: 'Content' }],
                },
            },
            {
                path: 'logs',
                name: 'Logs',
                component: () => import('@/pages/instance/Logs.vue'),
                meta: {
                    renderMode: 'fixed',
                    useRootContext: true,
                    breadcrumb: [{ name: 'Logs' }],
                },
            },
        ],
    },
]
