<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import Dialog from './Dialog.vue'
import Button from './Button.vue'

withDefaults(defineProps<{
  open: boolean
  title: string
  message: string
  details?: { label: string; value: string | number }[]
  loading?: boolean
  confirmText?: string
  cancelText?: string
  tone?: 'danger' | 'warning' | 'success' | 'primary'
}>(), {
  details: () => [],
  loading: false,
  tone: 'danger',
})

const emit = defineEmits<{
  'update:open': [value: boolean]
  confirm: []
}>()

const { t } = useI18n()
</script>

<template>
  <Dialog :open="open" :title="title" @update:open="emit('update:open', $event)">
    <p class="text-sm leading-6 text-muted">{{ message }}</p>
    <dl v-if="details.length" class="mt-4 space-y-1 rounded-lg border border-line bg-canvas px-3 py-2 text-sm">
      <div v-for="item in details" :key="item.label" class="flex justify-between gap-4">
        <dt class="text-faint">{{ item.label }}</dt>
        <dd class="font-medium text-ink">{{ item.value }}</dd>
      </div>
    </dl>
    <template #footer>
      <Button variant="secondary" :disabled="loading" @click="emit('update:open', false)">
        {{ cancelText || t('common.cancel') }}
      </Button>
      <Button :variant="tone === 'success' || tone === 'primary' ? 'primary' : tone" :loading="loading" @click="emit('confirm')">
        {{ confirmText || t('common.confirm') }}
      </Button>
    </template>
  </Dialog>
</template>
