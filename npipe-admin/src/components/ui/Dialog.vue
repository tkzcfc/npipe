<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useFocusTrap } from '@/composables/useFocusTrap'
import Button from './Button.vue'
import Icon from './Icon.vue'

const props = defineProps<{
  open: boolean
  title: string
  description?: string
  width?: string
}>()

const emit = defineEmits<{
  'update:open': [value: boolean]
  close: []
}>()

const { t } = useI18n()
const panel = ref<HTMLElement | null>(null)
const isOpen = computed(() => props.open)
useFocusTrap(panel, isOpen)

function close() {
  emit('update:open', false)
  emit('close')
}

function onKey(e: KeyboardEvent) {
  if (e.key === 'Escape' && props.open) close()
}

watch(() => props.open, (open) => {
  document.body.style.overflow = open ? 'hidden' : ''
})
onMounted(() => document.addEventListener('keydown', onKey))
onUnmounted(() => {
  document.removeEventListener('keydown', onKey)
  document.body.style.overflow = ''
})
</script>

<template>
  <Teleport to="body">
    <div v-if="open" class="fixed inset-0 z-[80] flex items-center justify-center p-4">
      <div class="absolute inset-0 bg-black/55 backdrop-blur-[2px]" @click="close" />
      <div
        ref="panel"
        role="dialog"
        aria-modal="true"
        :aria-label="title"
        class="relative z-10 w-full rounded-xl border border-line bg-surface shadow-lift"
        :style="{ maxWidth: width || '28rem' }"
      >
        <div class="flex items-start justify-between gap-4 border-b border-line px-5 py-4">
          <div>
            <h2 class="text-base font-semibold text-ink">{{ title }}</h2>
            <p v-if="description" class="mt-1 text-sm text-muted">{{ description }}</p>
          </div>
          <Button variant="ghost" size="sm" icon="close" :aria-label="t('common.close')" @click="close" />
        </div>
        <div class="px-5 py-4">
          <slot />
        </div>
        <div v-if="$slots.footer" class="flex justify-end gap-2 border-t border-line px-5 py-3">
          <slot name="footer" />
        </div>
      </div>
    </div>
  </Teleport>
</template>
