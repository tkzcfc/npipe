<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import Icon from './Icon.vue'
import Button from './Button.vue'

withDefaults(defineProps<{
  title?: string
  description?: string
  icon?: string
  retry?: boolean
}>(), { icon: 'log' })

const emit = defineEmits<{ retry: [] }>()
const { t } = useI18n()
</script>

<template>
  <div class="flex flex-col items-center justify-center px-6 py-16 text-center">
    <div class="mb-3 flex h-12 w-12 items-center justify-center rounded-2xl bg-accent-dim text-accent">
      <Icon :name="icon" :size="22" />
    </div>
    <p class="text-sm font-medium text-ink">{{ title || t('common.empty') }}</p>
    <p v-if="description" class="mt-1 max-w-sm text-sm text-muted">{{ description }}</p>
    <div class="mt-4 flex items-center gap-2">
      <Button v-if="retry" variant="primary" size="sm" icon="refresh" @click="emit('retry')">{{ t('common.retry') }}</Button>
      <slot />
    </div>
  </div>
</template>
