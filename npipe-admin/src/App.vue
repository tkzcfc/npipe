<template>
  <el-config-provider :locale="elLocale">
    <router-view />
  </el-config-provider>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAppStore } from '@/stores/app'
import zhCn from 'element-plus/es/locale/lang/zh-cn'
import en from 'element-plus/es/locale/lang/en'

const { locale } = useI18n()
const appStore = useAppStore()

// Element Plus locale 跟随 vue-i18n locale
const elLocale = computed(() => (locale.value === 'zh-CN' ? zhCn : en))

// Apply theme on startup
document.documentElement.setAttribute('data-theme', appStore.theme)
if (appStore.theme === 'dark') {
  document.documentElement.classList.add('dark')
}
</script>

