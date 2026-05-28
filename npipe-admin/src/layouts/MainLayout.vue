<template>
  <div class="main-layout" :class="{ 'sidebar-collapsed': appStore.sidebarCollapsed }">
    <!-- Sidebar -->
    <aside class="sidebar">
      <div class="sidebar-logo">
        <span class="logo-icon">⚡</span>
        <span v-show="!appStore.sidebarCollapsed" class="logo-text">npipe Console</span>
      </div>

      <el-menu
        :default-active="route.path"
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
      <!-- Header -->
      <header class="header">
        <div class="header-left">
          <el-breadcrumb separator="/">
            <el-breadcrumb-item :to="{ path: '/' }">{{ $t('layout.home') }}</el-breadcrumb-item>
            <el-breadcrumb-item v-if="route.meta.title">
              {{ route.meta.title }}
            </el-breadcrumb-item>
          </el-breadcrumb>
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

          <!-- Language Switcher -->
          <el-dropdown @command="onSwitchLang">
            <button class="icon-btn">
              <el-icon><Flag /></el-icon>
            </button>
            <template #dropdown>
              <el-dropdown-menu>
                <el-dropdown-item command="zh-CN" :class="{ 'is-active': currentLang === 'zh-CN' }">
                  🇨🇳 中文
                </el-dropdown-item>
                <el-dropdown-item command="en-US" :class="{ 'is-active': currentLang === 'en-US' }">
                  🇺🇸 English
                </el-dropdown-item>
              </el-dropdown-menu>
            </template>
          </el-dropdown>

          <el-dropdown @command="onCommand">
            <div class="user-avatar">
              <el-icon><UserFilled /></el-icon>
              <span class="username">Admin</span>
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

      <!-- Page content -->
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
import { setLanguage } from '@/locales'
import { ElMessageBox, ElMessage } from 'element-plus'

const { t, locale } = useI18n()
const route = useRoute()
const router = useRouter()
const appStore = useAppStore()
const authStore = useAuthStore()

const currentLang = computed(() => locale.value)

const menuItems = computed(() => [
  { path: '/dashboard', title: t('dashboard.title'),  icon: 'Odometer' },
  { path: '/players',   title: t('player.title'),     icon: 'User' },
  { path: '/tunnels',   title: t('tunnel.title'),     icon: 'Connection' },
])

function onSwitchLang(lang: 'zh-CN' | 'en-US') {
  setLanguage(lang)
}

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
  background: var(--bg-sidebar);
  border-right: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
  transition: width .25s ease, min-width .25s ease;
  overflow: hidden;
  position: relative;
  z-index: 10;
}

.sidebar-logo {
  height: 56px;
  display: flex;
  align-items: center;
  padding: 0 20px;
  gap: 10px;
  border-bottom: 1px solid rgba(255,255,255,.06);
  white-space: nowrap;
  overflow: hidden;

  .logo-icon { font-size: 22px; flex-shrink: 0; }

  .logo-text {
    font-size: 15px;
    font-weight: 700;
    color: #fff;
    letter-spacing: .5px;
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
    border-radius: 8px;
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
  height: 56px;
  background: var(--bg-header);
  border-bottom: 1px solid var(--border-color);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 20px;
  flex-shrink: 0;
  box-shadow: var(--shadow-sm);
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

