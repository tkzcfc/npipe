<script setup lang="ts">
import { computed } from 'vue'
import { cn } from '@/utils/cn'
import Icon from './Icon.vue'

const props = withDefaults(defineProps<{
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger' | 'success' | 'warning'
  size?: 'sm' | 'md' | 'lg'
  loading?: boolean
  disabled?: boolean
  type?: 'button' | 'submit'
  block?: boolean
  icon?: string
}>(), {
  variant: 'secondary',
  size: 'md',
  type: 'button',
})

const classes = computed(() => cn(
  'inline-flex items-center justify-center gap-1.5 rounded-lg font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-50',
  props.size === 'sm' && 'h-8 px-2.5 text-[13px]',
  props.size === 'md' && 'h-9 px-3.5 text-sm',
  props.size === 'lg' && 'h-11 px-4 text-[15px]',
  props.block && 'w-full',
  props.variant === 'primary' && 'bg-accent text-accent-fg hover:bg-[var(--accent-hover)] shadow-soft',
  props.variant === 'secondary' && 'border border-line bg-surface text-ink hover:bg-[var(--accent-bg)]',
  props.variant === 'ghost' && 'text-muted hover:bg-[var(--accent-bg)] hover:text-ink',
  props.variant === 'danger' && 'bg-[var(--danger)] text-white hover:opacity-90',
  props.variant === 'success' && 'bg-accent text-accent-fg hover:bg-[var(--accent-hover)]',
  props.variant === 'warning' && 'bg-[var(--warning)] text-[#1a1203] hover:opacity-90',
))
</script>

<template>
  <button :type="type" :class="classes" :disabled="disabled || loading">
    <span v-if="loading" class="h-3.5 w-3.5 animate-spin rounded-full border-2 border-current border-t-transparent" />
    <Icon v-else-if="icon" :name="icon" :size="size === 'sm' ? 14 : 16" />
    <slot />
  </button>
</template>
