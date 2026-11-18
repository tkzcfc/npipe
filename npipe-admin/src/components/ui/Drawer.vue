<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useFocusTrap } from '@/composables/useFocusTrap'
import Button from './Button.vue'

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
    <div v-if="open" class="fixed inset-0 z-[80]">
      <div class="absolute inset-0 bg-black/50 backdrop-blur-[2px]" @click="close" />
      <aside
        ref="panel"
        role="dialog"
        aria-modal="true"
        :aria-label="title"
        class="absolute inset-y-0 right-0 flex w-full max-w-full flex-col border-l border-line bg-surface shadow-lift"
        :style="{ width: width || '36rem' }"
      >
        <div class="flex items-start justify-between gap-4 border-b border-line px-5 py-4">
          <div>
            <h2 class="text-base font-semibold text-ink">{{ title }}</h2>
            <p v-if="description" class="mt-1 text-sm text-muted">{{ description }}</p>
          </div>
          <Button variant="ghost" size="sm" icon="close" :aria-label="t('common.close')" @click="close" />
        </div>
        <div class="min-h-0 flex-1 overflow-y-auto px-5 py-4">
          <slot />
        </div>
        <div v-if="$slots.footer" class="border-t border-line px-5 py-3">
          <slot name="footer" />
        </div>
      </aside>
    </div>
  </Teleport>
</template>
