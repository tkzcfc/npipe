<script setup lang="ts">
import { cn } from '@/utils/cn'

withDefaults(defineProps<{
  tone?: 'neutral' | 'success' | 'warning' | 'danger' | 'accent' | 'info'
}>(), { tone: 'neutral' })
</script>

<template>
  <span
    :class="cn(
      'inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[11px] font-semibold tracking-wide',
      tone === 'neutral' && 'bg-[var(--bg)] text-muted border border-line',
      tone === 'success' && 'bg-accent-dim text-[var(--success)]',
      tone === 'warning' && 'bg-[rgba(196,132,29,.12)] text-[var(--warning)]',
      tone === 'danger' && 'bg-[rgba(214,69,69,.12)] text-[var(--danger)]',
      tone === 'accent' && 'bg-accent-dim text-accent',
      tone === 'info' && 'bg-[rgba(47,127,191,.12)] text-[var(--info)]',
    )"
  >
    <slot />
  </span>
</template>
