<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useAppStore } from '@/stores/app'
import Segmented from './ui/Segmented.vue'

withDefaults(defineProps<{ compact?: boolean }>(), { compact: false })

const { t } = useI18n()
const app = useAppStore()
</script>

<template>
  <div>
    <p v-if="$attrs.labeled !== undefined || true" class="sr-only">{{ t('layout.language') }}</p>
    <Segmented
      :compact="compact"
      :model-value="app.locale"
      :options="[
        { label: t('layout.langZh'), value: 'zh-CN' },
        { label: t('layout.langEn'), value: 'en-US' },
      ]"
      @update:model-value="app.setLocale($event as 'zh-CN' | 'en-US')"
    />
  </div>
</template>
