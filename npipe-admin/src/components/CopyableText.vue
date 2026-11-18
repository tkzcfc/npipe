<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useToastStore } from '@/stores/toast'
import { copyText } from '@/utils/clipboard'
import { cn } from '@/utils/cn'
import Icon from '@/components/ui/Icon.vue'

const props = withDefaults(defineProps<{
  value?: string | number | null
  empty?: string
  mono?: boolean
}>(), {
  value: '',
  empty: '—',
  mono: true,
})

const { t } = useI18n()
const toast = useToastStore()
const text = () => String(props.value ?? '').trim()

async function copy() {
  const value = text()
  if (!value) return
  const ok = await copyText(value)
  if (ok) toast.success(t('common.copied'))
  else toast.error(t('common.copyFailed'))
}
</script>

<template>
  <span class="inline-flex max-w-full items-center gap-1">
    <span
      :class="cn('min-w-0 truncate', mono && 'mono text-[13px]')"
      :title="text() || undefined"
    >{{ text() || empty }}</span>
    <button
      v-if="text()"
      type="button"
      class="shrink-0 rounded p-0.5 text-faint hover:bg-canvas hover:text-ink"
      :aria-label="t('common.copy')"
      @click.stop="copy"
    >
      <Icon name="copy" :size="13" />
    </button>
  </span>
</template>
