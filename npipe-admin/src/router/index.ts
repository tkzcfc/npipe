import { createRouter, createWebHashHistory, type RouteRecordRaw } from 'vue-router'
import { useAuthStore } from '@/stores/auth'

const routes: RouteRecordRaw[] = [
  {
    path: '/login',
    name: 'Login',
    component: () => import('@/views/login/index.vue'),
    meta: { requiresAuth: false, title: '登录' },
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
        meta: { title: '运行概览', icon: 'Odometer', requiresAuth: true },
      },
      {
        path: 'players',
        name: 'Players',
        component: () => import('@/views/players/index.vue'),
        meta: { title: '用户管理', icon: 'User', requiresAuth: true },
      },
      {
        path: 'tunnels',
        name: 'Tunnels',
        component: () => import('@/views/tunnels/index.vue'),
        meta: { title: '隧道管理', icon: 'Connection', requiresAuth: true },
      },
      {
        path: 'logs',
        name: 'LoginLogs',
        component: () => import('@/views/logins/index.vue'),
        meta: { title: '登录日志', icon: 'Document', requiresAuth: true },
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

router.beforeEach(async (to) => {
  const authStore = useAuthStore()

  if (to.meta.requiresAuth !== false) {
    if (!authStore.isLoggedIn) {
      const ok = await authStore.checkAuth()
      if (!ok) {
        return { name: 'Login', query: { redirect: to.fullPath } }
      }
    }
  } else {
    // On login page, if already logged in, redirect to dashboard
    if (authStore.isLoggedIn) {
      return { name: 'Dashboard' }
    }
  }

  document.title = `${to.meta.title ? to.meta.title + ' - ' : ''}npipe Console`
})

export default router

