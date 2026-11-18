<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useAppStore } from '@/stores/app'
import type { ThemeMode } from '@/utils/theme'
import { cn } from '@/utils/cn'
import Icon from './ui/Icon.vue'

const { t } = useI18n()
const app = useAppStore()

const options: { value: ThemeMode; icon: string; label: () => string }[] = [
  { value: 'light', icon: 'sun', label: () => t('layout.themeLight') },
  { value: 'dark', icon: 'moon', label: () => t('layout.themeDark') },
  { value: 'system', icon: 'monitor', label: () => t('layout.themeSystem') },
]
</script>

<template>
  <div role="radiogroup" :aria-label="t('layout.theme')" class="inline-flex rounded-lg border border-line bg-canvas p-0.5">
    <button
      v-for="opt in options"
      :key="opt.value"
      type="button"
      role="radio"
      :aria-checked="app.themeMode === opt.value"
      :title="opt.label()"
      :aria-label="opt.label()"
      :class="cn(
        'flex h-8 w-8 items-center justify-center rounded-md transition-colors',
        app.themeMode === opt.value ? 'bg-surface text-accent shadow-soft' : 'text-muted hover:text-ink',
      )"
      @click="app.setThemeMode(opt.value)"
    >
      <Icon :name="opt.icon" :size="15" />
    </button>
  </div>
</template>
