<template>
  <el-config-provider :locale="elLocale">
    <transition name="route-mask">
      <div v-if="appStore.routeLoading" class="global-route-loading">
        <div class="global-route-card">
          <span class="global-route-spinner" />
          <span>{{ t('common.loadingPage') }}</span>
        </div>
      </div>
    </transition>
    <router-view />
  </el-config-provider>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAppStore } from '@/stores/app'
import zhCn from 'element-plus/es/locale/lang/zh-cn'
import en from 'element-plus/es/locale/lang/en'

const { locale, t } = useI18n()
const appStore = useAppStore()

// Element Plus locale 跟随 vue-i18n locale
const elLocale = computed(() => (locale.value === 'zh-CN' ? zhCn : en))

// Apply theme on startup
document.documentElement.setAttribute('data-theme', appStore.theme)
if (appStore.theme === 'dark') {
  document.documentElement.classList.add('dark')
}
</script>

<style scoped lang="scss">
.global-route-loading {
  position: fixed;
  inset: 0;
  z-index: 3000;
  display: flex;
  align-items: flex-start;
  justify-content: center;
  padding-top: 18px;
  pointer-events: none;
  background: linear-gradient(180deg, rgba(5, 8, 18, .24), rgba(5, 8, 18, 0));
}

.global-route-card {
  display: inline-flex;
  align-items: center;
  gap: 10px;
  min-height: 38px;
  padding: 0 14px;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: var(--bg-card);
  color: var(--text-primary);
  box-shadow: var(--shadow-md);
  font-size: 13px;
}

.global-route-spinner {
  width: 15px;
  height: 15px;
  border-radius: 50%;
  border: 2px solid rgba(91, 143, 249, .24);
  border-top-color: var(--accent);
  animation: global-route-spin .8s linear infinite;
}

.route-mask-enter-active,
.route-mask-leave-active {
  transition: opacity .16s ease, transform .16s ease;
}

.route-mask-enter-from,
.route-mask-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}

@keyframes global-route-spin {
  to { transform: rotate(360deg); }
}
</style>
