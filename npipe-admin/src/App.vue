<template>
  <router-view />
</template>

<script setup lang="ts">
import { useAppStore } from '@/stores/app'

const appStore = useAppStore()

// Apply theme on startup
document.documentElement.setAttribute('data-theme', appStore.theme)
if (appStore.theme === 'dark') {
  document.documentElement.classList.add('dark')
}
</script>

