import { createRouter, createWebHashHistory, type RouteRecordRaw } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { useAppStore } from '@/stores/app'
import i18n from '@/locales'

const routes: RouteRecordRaw[] = [
  {
    path: '/login',
    name: 'Login',
    component: () => import('@/views/login/index.vue'),
    meta: { requiresAuth: false, titleKey: 'login.title' },
  },
  {
    path: '/',
    component: () => import('@/layouts/MainLayout.vue'),
    meta: { requiresAuth: true },
    redirect: '/dashboard',
    children: [
      {
        path: 'dashboard',
        name: 'Dashboard',
        component: () => import('@/views/dashboard/index.vue'),
        meta: { titleKey: 'dashboard.title', requiresAuth: true, adminOnly: true },
      },
      {
        path: 'players',
        name: 'Players',
        component: () => import('@/views/players/index.vue'),
        meta: { titleKey: 'player.title', requiresAuth: true },
      },
      {
        path: 'players/:id',
        name: 'PlayerDetail',
        component: () => import('@/views/players/detail.vue'),
        meta: { titleKey: 'player.detailTitle', requiresAuth: true },
      },
      {
        path: 'tunnels',
        name: 'Tunnels',
        component: () => import('@/views/tunnels/index.vue'),
        meta: { titleKey: 'tunnel.title', requiresAuth: true },
      },
      {
        path: 'logs',
        name: 'LoginLogs',
        component: () => import('@/views/logins/index.vue'),
        meta: { titleKey: 'loginLog.title', requiresAuth: true },
      },
      {
        path: 'operations',
        name: 'OperationLogs',
        component: () => import('@/views/operations/index.vue'),
        meta: { titleKey: 'operationLog.title', requiresAuth: true, adminOnly: true },
      },
      {
        path: 'maintenance',
        name: 'Maintenance',
        component: () => import('@/views/maintenance/index.vue'),
        meta: { titleKey: 'maintenance.title', requiresAuth: true, adminOnly: true },
      },
    ],
  },
  {
    path: '/:pathMatch(.*)*',
    redirect: '/',
  },
]

const router = createRouter({
  history: createWebHashHistory(),
  routes,
})

router.beforeEach(async (to, from) => {
  const authStore = useAuthStore()
  const appStore = useAppStore()

  if (to.fullPath !== from.fullPath) {
    appStore.setRouteLoading(true)
  }

  if (to.meta.requiresAuth !== false) {
    if (!authStore.isLoggedIn) {
      const ok = await authStore.checkAuth()
      if (!ok) {
        return { name: 'Login', query: { redirect: to.fullPath } }
      }
    }
    if (to.meta.adminOnly && !authStore.isAdmin) {
      return { name: 'PlayerDetail', params: { id: authStore.currentUserId } }
    }
    if (to.name === 'Players' && !authStore.isAdmin) {
      return { name: 'PlayerDetail', params: { id: authStore.currentUserId } }
    }
    if (to.name === 'PlayerDetail' && !authStore.isAdmin && Number(to.params.id) !== authStore.currentUserId) {
      return { name: 'PlayerDetail', params: { id: authStore.currentUserId } }
    }
  } else if (authStore.isLoggedIn) {
    return authStore.isAdmin
      ? { name: 'Dashboard' }
      : { name: 'PlayerDetail', params: { id: authStore.currentUserId } }
  }

  const titleKey = typeof to.meta.titleKey === 'string' ? to.meta.titleKey : ''
  const page = titleKey ? String(i18n.global.t(titleKey)) : ''
  document.title = page ? `${page} · npipe Console` : 'npipe Console'
})

router.afterEach(() => {
  useAppStore().setRouteLoading(false)
})

router.onError(() => {
  useAppStore().setRouteLoading(false)
})

export default router
