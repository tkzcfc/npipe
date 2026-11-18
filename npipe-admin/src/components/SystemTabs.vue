<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { cn } from '@/utils/cn'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const authStore = useAuthStore()

const tabs = computed(() => [
  { path: '/logs', label: t('nav.loginLogs') },
  { path: '/operations', label: t('nav.operationLogs') },
  { path: '/maintenance', label: t('nav.maintenance') },
])

function isActive(path: string) {
  return route.path === path
}

async function go(path: string) {
  if (route.path === path) return
  await router.push(path)
}
</script>

<template>
  <div v-if="authStore.isAdmin" class="mb-4 flex flex-wrap gap-1 border-b border-line" role="tablist">
    <button
      v-for="tab in tabs"
      :key="tab.path"
      type="button"
      role="tab"
      :aria-selected="isActive(tab.path)"
      :class="cn(
        'border-b-2 px-3 py-2 text-sm transition-colors',
        isActive(tab.path)
          ? 'border-accent text-ink'
          : 'border-transparent text-muted hover:text-ink',
      )"
      @click="go(tab.path)"
    >
      {{ tab.label }}
    </button>
  </div>
</template>
