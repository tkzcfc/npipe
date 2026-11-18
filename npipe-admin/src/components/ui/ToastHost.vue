<script setup lang="ts">
import { useToastStore } from '@/stores/toast'
import { cn } from '@/utils/cn'
import Icon from './Icon.vue'

const toast = useToastStore()

function icon(kind: string) {
  if (kind === 'success') return 'success'
  if (kind === 'warning') return 'warning'
  if (kind === 'error') return 'error'
  return 'info'
}
</script>

<template>
  <div class="pointer-events-none fixed right-4 top-4 z-[100] flex w-[min(22rem,calc(100vw-2rem))] flex-col gap-2">
    <div
      v-for="item in toast.items"
      :key="item.id"
      role="status"
      :class="cn(
        'pointer-events-auto flex items-start gap-2 rounded-xl border bg-surface px-3 py-2.5 text-sm shadow-lift',
        item.kind === 'success' && 'border-[var(--success)]/30',
        item.kind === 'error' && 'border-[var(--danger)]/40',
        item.kind === 'warning' && 'border-[var(--warning)]/40',
        item.kind === 'info' && 'border-line',
      )"
    >
      <Icon :name="icon(item.kind)" :size="16" class="mt-0.5 text-current" />
      <p class="flex-1 leading-5 text-ink">{{ item.message }}</p>
      <button type="button" class="text-faint hover:text-ink" @click="toast.dismiss(item.id)">
        <Icon name="close" :size="14" />
      </button>
    </div>
  </div>
</template>
