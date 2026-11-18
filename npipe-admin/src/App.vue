<script setup lang="ts">
import { onMounted } from 'vue'
import { useAppStore } from '@/stores/app'
import ToastHost from '@/components/ui/ToastHost.vue'

const appStore = useAppStore()
onMounted(() => appStore.initThemeListener())
</script>

<template>
  <router-view />
  <ToastHost />
</template>
