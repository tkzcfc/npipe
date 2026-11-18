<script setup lang="ts">
import { cn } from '@/utils/cn'

defineProps<{
  modelValue?: string | number
  disabled?: boolean
  id?: string
  invalid?: boolean
}>()

const emit = defineEmits<{ 'update:modelValue': [value: string] }>()
</script>

<template>
  <select
    :id="id"
    :value="modelValue"
    :disabled="disabled"
    :class="cn(
      'h-10 w-full rounded-lg border bg-canvas px-3 text-sm text-ink shadow-soft',
      'focus:border-accent focus:outline-none focus:ring-2 focus:ring-accent/30',
      invalid ? 'border-[var(--danger)]' : 'border-line',
    )"
    @change="emit('update:modelValue', ($event.target as HTMLSelectElement).value)"
  >
    <slot />
  </select>
</template>
