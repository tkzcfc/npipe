<template>
  <div class="main-layout" :class="{ 'sidebar-collapsed': appStore.sidebarCollapsed }">
    <aside class="sidebar">
      <div class="sidebar-logo">
        <span class="logo-icon"><el-icon><Connection /></el-icon></span>
        <span v-show="!appStore.sidebarCollapsed" class="logo-text">npipe Console</span>
      </div>

      <el-menu
        :default-active="activeMenu"
        :collapse="appStore.sidebarCollapsed"
        :collapse-transition="false"
        router
        class="sidebar-menu"
      >
        <el-menu-item
          v-for="item in menuItems"
          :key="item.path"
          :index="item.path"
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
        <router-view v-slot="{ Component }">
          <transition name="fade" mode="out-in">
            <component :is="Component" />
          </transition>
        </router-view>
      </main>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useAppStore } from '@/stores/app'
import { useAuthStore } from '@/stores/auth'
import { ElMessageBox, ElMessage } from 'element-plus'
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

async function onCommand(cmd: string) {
  if (cmd === 'logout') {
    await ElMessageBox.confirm(t('layout.logoutConfirm'), t('layout.logoutTitle'), {
      confirmButtonText: t('common.ok'),
      cancelButtonText: t('common.cancel'),
      type: 'warning',
    })
    await authStore.logout()
    ElMessage.success(t('layout.logoutSuccess'))
    router.push('/login')
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

    &.is-active {
      background: rgba(91,143,249,.22) !important;
      color: #5b8ff9 !important;

      .el-icon { color: #5b8ff9; }
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
}

// ── Transition ───────────────────────────────────────────────────────────────
.fade-enter-active, .fade-leave-active { transition: opacity .2s ease; }
.fade-enter-from, .fade-leave-to { opacity: 0; }
</style>

