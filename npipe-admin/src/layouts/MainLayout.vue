<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import { useAppStore } from '@/stores/app'
import { useAuthStore } from '@/stores/auth'
import { useToastStore } from '@/stores/toast'
import { useScrollLock } from '@/composables/useScrollLock'
import { cn } from '@/utils/cn'
import Button from '@/components/ui/Button.vue'
import Icon from '@/components/ui/Icon.vue'
import ConfirmDialog from '@/components/ui/ConfirmDialog.vue'
import LanguageSwitch from '@/components/LanguageSwitch.vue'
import ThemeSwitch from '@/components/ThemeSwitch.vue'
import AccountMenu from '@/components/AccountMenu.vue'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const appStore = useAppStore()
const authStore = useAuthStore()
const toast = useToastStore()

const mobileOpen = ref(false)
const logoutOpen = ref(false)
const logoutLoading = ref(false)
useScrollLock(mobileOpen)

const isMobile = () => window.innerWidth <= 768
function onResize() {
  if (!isMobile()) mobileOpen.value = false
}
onMounted(() => window.addEventListener('resize', onResize))
onUnmounted(() => window.removeEventListener('resize', onResize))
watch(() => route.fullPath, () => { mobileOpen.value = false })

const systemPaths = ['/logs', '/operations', '/maintenance']

const menuItems = computed(() => [
  ...(authStore.isAdmin ? [{ path: '/dashboard', title: t('nav.dashboard'), icon: 'activity' }] : []),
  {
    path: authStore.isAdmin ? '/players' : `/players/${authStore.currentUserId}`,
    title: authStore.isAdmin ? t('nav.players') : t('nav.myAccount'),
    icon: 'user',
  },
  { path: '/tunnels', title: t('nav.tunnels'), icon: 'tunnel' },
  ...(authStore.isAdmin ? [{ path: '/logs', title: t('nav.system'), icon: 'shield' }] : []),
])

const activePath = computed(() => {
  if (route.path.startsWith('/players')) return authStore.isAdmin ? '/players' : `/players/${authStore.currentUserId}`
  if (route.path.startsWith('/tunnels')) return '/tunnels'
  if (systemPaths.includes(route.path)) return '/logs'
  return route.path
})

async function go(path: string) {
  if (path === route.path) return
  mobileOpen.value = false
  await router.push(path)
}

async function goAccount() {
  mobileOpen.value = false
  if (authStore.currentUserId) {
    await router.push({ name: 'PlayerDetail', params: { id: authStore.currentUserId } })
    return
  }
  if (authStore.isAdmin) await router.push({ name: 'Players' })
}

async function goLoginLogs() {
  mobileOpen.value = false
  await router.push({ name: 'LoginLogs' })
}

async function confirmLogout() {
  logoutLoading.value = true
  try {
    await authStore.logout()
    toast.info(t('layout.logoutSuccess'))
    logoutOpen.value = false
    await router.replace({ name: 'Login' })
  } finally {
    logoutLoading.value = false
  }
}

const pageTitle = computed(() => {
  const key = typeof route.meta.titleKey === 'string' ? route.meta.titleKey : ''
  return key ? t(key) : ''
})
</script>

<template>
  <div
    class="min-h-full bg-canvas text-ink"
    :style="{ '--current-sidebar': appStore.sidebarCollapsed ? 'var(--sidebar-w-collapsed)' : 'var(--sidebar-w)' }"
  >
    <a href="#main" class="sr-only focus:not-sr-only focus:absolute focus:left-3 focus:top-3 focus:z-[90] focus:rounded-lg focus:bg-surface focus:px-3 focus:py-2">
      {{ t('common.skipToContent') }}
    </a>

    <div v-if="mobileOpen" class="fixed inset-0 z-40 bg-black/50 md:hidden" @click="mobileOpen = false" />

    <aside
      :class="cn(
        'fixed inset-y-0 left-0 z-50 flex flex-col border-r border-line bg-sidebar text-[var(--text-sidebar)] transition-all duration-200',
        'max-md:-translate-x-full',
        mobileOpen && 'max-md:translate-x-0',
      )"
      :style="{ width: appStore.sidebarCollapsed ? 'var(--sidebar-w-collapsed)' : 'var(--sidebar-w)' }"
    >
      <div
        :class="cn(
          'flex h-14 shrink-0 items-center',
          appStore.sidebarCollapsed ? 'justify-center px-2' : 'gap-2 px-3',
        )"
      >
        <img src="/logo.svg" alt="" class="h-8 w-8 shrink-0 rounded-lg" />
        <div v-if="!appStore.sidebarCollapsed" class="min-w-0 flex-1">
          <p class="truncate text-sm font-semibold tracking-wide">{{ t('layout.appName') }}</p>
          <p class="ui-kicker !text-[var(--text-sidebar-muted)]">{{ t('layout.appTag') }}</p>
        </div>
      </div>

      <nav class="flex-1 space-y-1 overflow-y-auto px-2 pt-3" :aria-label="t('layout.appName')">
        <button
          v-for="item in menuItems"
          :key="item.path"
          type="button"
          :class="cn(
            'flex h-10 w-full items-center rounded-lg text-left text-sm transition-colors',
            appStore.sidebarCollapsed ? 'justify-center px-0' : 'gap-3 px-2.5',
            activePath === item.path
              ? 'bg-accent-dim text-accent'
              : 'text-[var(--text-sidebar-muted)] hover:bg-[var(--bg-sidebar-hover)] hover:text-[var(--text-sidebar)]',
          )"
          :title="item.title"
          @click="go(item.path)"
        >
          <Icon :name="item.icon" :size="18" />
          <span v-if="!appStore.sidebarCollapsed" class="truncate">{{ item.title }}</span>
        </button>
      </nav>

      <div class="mt-auto hidden shrink-0 overflow-visible border-t border-line p-2 md:block">
        <div
          :class="cn(
            'group relative overflow-visible',
            appStore.sidebarCollapsed ? 'flex justify-center' : 'flex',
          )"
        >
          <button
            type="button"
            class="flex h-10 w-10 items-center justify-center rounded-lg text-[var(--text-sidebar-muted)] transition-colors hover:bg-[var(--bg-sidebar-hover)] hover:text-[var(--text-sidebar)] focus-visible:bg-[var(--bg-sidebar-hover)] focus-visible:text-[var(--text-sidebar)]"
            :title="appStore.sidebarCollapsed ? t('layout.expand') : t('layout.collapse')"
            :aria-label="appStore.sidebarCollapsed ? t('layout.expand') : t('layout.collapse')"
            :aria-expanded="!appStore.sidebarCollapsed"
            @click="appStore.toggleSidebar()"
          >
            <Icon name="menu" :size="18" />
          </button>
          <span
            role="tooltip"
            class="pointer-events-none absolute z-[60] hidden whitespace-nowrap rounded-md border border-line bg-surface px-2 py-1 text-xs text-ink shadow-soft group-hover:block group-focus-within:block"
            :class="appStore.sidebarCollapsed ? 'left-full top-1/2 ml-1.5 -translate-y-1/2' : 'bottom-full left-0 mb-1.5'"
          >
            {{ appStore.sidebarCollapsed ? t('layout.expand') : t('layout.collapse') }}
          </span>
        </div>
      </div>
    </aside>

    <div class="flex min-h-full flex-col" style="margin-left: 0;">
      <div class="flex min-h-full flex-col md:ml-[var(--current-sidebar)]">
        <header class="sticky top-0 z-30 flex h-14 items-center gap-3 border-b border-line bg-[var(--bg-header)]/90 px-4 backdrop-blur">
          <div class="flex min-w-0 flex-1 items-center gap-2">
            <Button class="md:hidden" variant="ghost" size="sm" icon="menu" :aria-label="t('layout.openMenu')" @click="mobileOpen = true" />
            <p class="min-w-0 truncate text-sm font-semibold text-ink">{{ pageTitle }}</p>
          </div>
          <div class="flex shrink-0 items-center gap-2">
            <LanguageSwitch compact />
            <ThemeSwitch />
            <AccountMenu @account="goAccount" @login-logs="goLoginLogs" @logout="logoutOpen = true" />
          </div>
        </header>

        <div v-if="appStore.routeLoading" class="h-0.5 overflow-hidden bg-line">
          <div class="h-full w-1/3 animate-pulse bg-accent" />
        </div>

        <main id="main" class="flex-1 px-4 py-5 sm:px-6">
          <router-view />
        </main>
      </div>
    </div>

    <ConfirmDialog
      :open="logoutOpen"
      :title="t('layout.logoutTitle')"
      :message="t('layout.logoutConfirm')"
      :loading="logoutLoading"
      :confirm-text="t('layout.logout')"
      tone="warning"
      @update:open="logoutOpen = $event"
      @confirm="confirmLogout"
    />
  </div>
</template>
