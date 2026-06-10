<template>
  <div class="main-layout" :class="{ 'sidebar-collapsed': appStore.sidebarCollapsed, 'mobile-open': mobileOpen }">
    <div class="mobile-backdrop" @click="mobileOpen = false" />
    <aside class="sidebar">
      <div class="sidebar-logo">
        <span class="logo-icon"><el-icon><Connection /></el-icon></span>
        <span v-show="!appStore.sidebarCollapsed" class="logo-text">npipe Console</span>
      </div>

      <el-menu
        :default-active="activeMenu"
        :collapse="appStore.sidebarCollapsed"
        :collapse-transition="false"
        class="sidebar-menu"
      >
        <el-menu-item
          v-for="item in menuItems"
          :key="item.path"
          :index="item.path"
          :class="{ 'is-navigating': pendingPath === item.path }"
          @click="navigateTo(item.path)"
        >
          <el-icon><component :is="item.icon" /></el-icon>
          <template #title>{{ item.title }}</template>
        </el-menu-item>
      </el-menu>

      <div class="sidebar-footer">
        <el-tooltip :content="appStore.sidebarCollapsed ? $t('layout.expand') : $t('layout.collapse')" placement="right">
          <button class="collapse-btn" @click="appStore.toggleSidebar">
            <el-icon>
              <Expand v-if="appStore.sidebarCollapsed" />
              <Fold v-else />
            </el-icon>
          </button>
        </el-tooltip>
      </div>
    </aside>

    <!-- Main content -->
    <div class="main-wrapper">
      <header class="header">
        <div class="header-left">
          <button class="mobile-menu-btn" @click="mobileOpen = !mobileOpen">
            <el-icon><Expand /></el-icon>
          </button>
          <div class="route-title">{{ route.meta.title }}</div>
        </div>
        <div class="header-right">
          <el-tooltip :content="appStore.theme === 'dark' ? $t('layout.themeLight') : $t('layout.themeDark')">
            <button class="icon-btn" @click="appStore.toggleTheme">
              <el-icon>
                <Sunny v-if="appStore.theme === 'dark'" />
                <Moon v-else />
              </el-icon>
            </button>
          </el-tooltip>

          <el-dropdown @command="onCommand">
            <div class="user-avatar">
              <el-icon><UserFilled /></el-icon>
              <span class="username">{{ authStore.displayName }}</span>
              <span class="role-badge">{{ authStore.isAdmin ? $t('layout.admin') : $t('layout.user') }}</span>
              <el-icon class="arrow"><ArrowDown /></el-icon>
            </div>
            <template #dropdown>
              <el-dropdown-menu>
                <el-dropdown-item command="logout">
                  <el-icon><SwitchButton /></el-icon> {{ $t('layout.logout') }}
                </el-dropdown-item>
              </el-dropdown-menu>
            </template>
          </el-dropdown>
        </div>
      </header>

      <main class="content">
        <div v-if="isRouteLoading" class="route-loading-bar" />
        <router-view v-slot="{ Component }">
          <transition name="fade" mode="out-in">
            <component :is="Component" />
          </transition>
        </router-view>
      </main>
    </div>

    <ConfirmAction
      v-model:visible="logoutDialog.visible"
      :title="$t('layout.logoutTitle')"
      :message="$t('layout.logoutConfirm')"
      :loading="logoutDialog.loading"
      :confirm-text="$t('layout.logout')"
      :cancel-text="$t('common.cancel')"
      confirm-type="warning"
      @confirm="handleLogoutConfirm"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch, onMounted, onUnmounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useAppStore } from '@/stores/app'
import { useAuthStore } from '@/stores/auth'
import { ElMessage } from 'element-plus'
import ConfirmAction from '@/components/ConfirmAction.vue'
import {
  ArrowDown,
  Connection,
  Document,
  Expand,
  Fold,
  Moon,
  Odometer,
  Sunny,
  SwitchButton,
  Tickets,
  Tools,
  User,
  UserFilled,
} from '@element-plus/icons-vue'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const appStore = useAppStore()
const authStore = useAuthStore()
const pendingPath = ref('')
const isRouteLoading = ref(false)
const mobileOpen = ref(false)
const logoutDialog = ref({
  visible: false,
  loading: false,
})

const isMobile = () => window.innerWidth <= 768
function onResize() {
  if (!isMobile()) mobileOpen.value = false
}
onMounted(() => window.addEventListener('resize', onResize))
onUnmounted(() => window.removeEventListener('resize', onResize))

watch(
  () => route.fullPath,
  () => {
    pendingPath.value = ''
    isRouteLoading.value = false
  },
)

const activeMenu = computed(() => {
  if (route.path.startsWith('/players')) return authStore.isAdmin ? '/players' : `/players/${authStore.currentUserId}`
  if (route.path.startsWith('/tunnels')) return '/tunnels'
  return route.path
})

const menuItems = computed(() => [
  ...(authStore.isAdmin ? [{ path: '/dashboard', title: t('dashboard.title'), icon: Odometer }] : []),
  { path: authStore.isAdmin ? '/players' : `/players/${authStore.currentUserId}`, title: authStore.isAdmin ? t('player.title') : t('player.myAccount'), icon: User },
  { path: '/tunnels', title: t('tunnel.title'), icon: Connection },
  { path: '/logs', title: t('loginLog.title'), icon: Document },
  ...(authStore.isAdmin ? [{ path: '/operations', title: t('operationLog.title'), icon: Tickets }] : []),
  ...(authStore.isAdmin ? [{ path: '/maintenance', title: t('maintenance.title'), icon: Tools }] : []),
])

async function navigateTo(path: string) {
  if (path === route.path || pendingPath.value === path) return
  pendingPath.value = path
  isRouteLoading.value = true
  mobileOpen.value = false
  try {
    await router.push(path)
  } catch {
    pendingPath.value = ''
    isRouteLoading.value = false
  }
}

async function onCommand(cmd: string) {
  if (cmd === 'logout') {
    logoutDialog.value.visible = true
  }
}

async function handleLogoutConfirm() {
  logoutDialog.value.loading = true
  try {
    await authStore.logout()
    ElMessage.success(t('layout.logoutSuccess'))
    logoutDialog.value.visible = false
    await router.push('/login')
  } finally {
    logoutDialog.value.loading = false
  }
}
</script>

<style scoped lang="scss">
.main-layout {
  display: flex;
  height: 100vh;
  overflow: hidden;

  --sidebar-w: 220px;

  &.sidebar-collapsed {
    --sidebar-w: 64px;
  }
}

// ── Sidebar ──────────────────────────────────────────────────────────────────
.sidebar {
  width: var(--sidebar-w);
  min-width: var(--sidebar-w);
  height: 100vh;
  background: linear-gradient(180deg, #111827 0%, #0f1724 100%);
  border-right: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
  transition: width .25s ease, min-width .25s ease;
  overflow: hidden;
  position: relative;
  z-index: 10;
}

.sidebar-logo {
  height: 64px;
  display: flex;
  align-items: center;
  padding: 0 20px;
  gap: 10px;
  border-bottom: 1px solid rgba(255,255,255,.06);
  white-space: nowrap;
  overflow: hidden;

  .logo-icon {
    width: 30px;
    height: 30px;
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #8ab4ff;
    background: rgba(91,143,249,.14);
    flex-shrink: 0;
  }

  .logo-text {
    font-size: 15px;
    font-weight: 700;
    color: #fff;
    letter-spacing: 0;
  }
}

.sidebar-menu {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  border: none;
  background: transparent;
  padding-top: 8px;

  :deep(.el-menu-item) {
    height: 42px;
    line-height: 42px;
    margin: 2px 10px;
    border-radius: 7px;
    color: rgba(255,255,255,.55);
    transition: all .18s;

    &:hover {
      background: rgba(255,255,255,.07) !important;
      color: rgba(255,255,255,.9);
    }

    &.is-active,
    &.is-navigating {
      background: rgba(91,143,249,.22) !important;
      color: #5b8ff9 !important;

      .el-icon { color: #5b8ff9; }
    }

    &.is-navigating {
      cursor: wait;
      opacity: .82;
    }
  }
}

.sidebar-footer {
  padding: 12px 14px;
  border-top: 1px solid rgba(255,255,255,.05);
}

.collapse-btn {
  width: 100%;
  background: rgba(255,255,255,.05);
  border: 1px solid rgba(255,255,255,.07);
  border-radius: 7px;
  color: rgba(255,255,255,.4);
  cursor: pointer;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all .2s;

  &:hover {
    background: rgba(255,255,255,.1);
    color: #fff;
  }
}

// ── Main wrapper ────────────────────────────────────────────────────────────
.main-wrapper {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

// ── Header ──────────────────────────────────────────────────────────────────
.header {
  height: 64px;
  background: var(--bg-header);
  border-bottom: 1px solid var(--border-color);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 24px;
  flex-shrink: 0;
  box-shadow: var(--shadow-sm);
}

.route-title {
  font-size: 16px;
  font-weight: 700;
  color: var(--text-primary);
}

.header-right {
  display: flex;
  align-items: center;
  gap: 12px;
}

.icon-btn {
  background: none;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  width: 32px; height: 32px;
  cursor: pointer;
  color: var(--text-secondary);
  display: flex; align-items: center; justify-content: center;
  transition: all .2s;

  &:hover { border-color: var(--accent); color: var(--accent); }
}

.user-avatar {
  display: flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  padding: 4px 10px;
  border-radius: 6px;
  border: 1px solid var(--border-color);
  color: var(--text-secondary);
  transition: all .2s;
  font-size: 13px;

  &:hover { border-color: var(--accent); color: var(--accent); }

  .username { font-weight: 500; }
  .role-badge {
    font-size: 11px;
    line-height: 1;
    padding: 3px 5px;
    border-radius: 5px;
    color: var(--accent);
    background: rgba(91,143,249,.12);
  }
  .arrow { font-size: 12px; }
}

// ── Content ──────────────────────────────────────────────────────────────────
.content {
  flex: 1;
  overflow-y: auto;
  background: var(--bg-primary);
  position: relative;
}

.route-loading-bar {
  position: sticky;
  top: 0;
  z-index: 20;
  height: 2px;
  overflow: hidden;
  background: rgba(91,143,249,.14);

  &::after {
    content: '';
    position: absolute;
    top: 0;
    bottom: 0;
    left: -35%;
    width: 35%;
    background: var(--accent);
    animation: route-loading 1s ease-in-out infinite;
  }
}

// ── Transition ───────────────────────────────────────────────────────────────
.fade-enter-active, .fade-leave-active { transition: opacity .2s ease; }
.fade-enter-from, .fade-leave-to { opacity: 0; }

@keyframes route-loading {
  0% { left: -35%; }
  100% { left: 100%; }
}

// ── Mobile ───────────────────────────────────────────────────────────────────
.mobile-menu-btn {
  display: none;
  background: none;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  width: 32px; height: 32px;
  cursor: pointer;
  color: var(--text-secondary);
  align-items: center;
  justify-content: center;
  transition: all .2s;
  flex-shrink: 0;

  &:hover { border-color: var(--accent); color: var(--accent); }
}

.mobile-backdrop {
  display: none;
}

@media (max-width: 768px) {
  .main-layout {
    --sidebar-w: 0px;

    &.sidebar-collapsed {
      --sidebar-w: 0px;
    }
  }

  .sidebar {
    position: fixed;
    left: 0; top: 0;
    width: 220px !important;
    min-width: 220px !important;
    transform: translateX(-100%);
    transition: transform .25s ease;
    z-index: 1000;
  }

  .sidebar-footer {
    display: none;
  }

  .mobile-open {
    .sidebar {
      transform: translateX(0);
    }

    .mobile-backdrop {
      display: block;
      position: fixed;
      inset: 0;
      background: rgba(0, 0, 0, .5);
      z-index: 999;
    }
  }

  .mobile-menu-btn {
    display: flex;
  }

  .header {
    padding: 0 12px;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .user-avatar {
    .username, .role-badge, .arrow {
      display: none;
    }
  }
}
</style>
