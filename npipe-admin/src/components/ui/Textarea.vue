<script setup lang="ts">
import { cn } from '@/utils/cn'

defineProps<{
  modelValue?: string
  placeholder?: string
  disabled?: boolean
  rows?: number
  invalid?: boolean
  id?: string
}>()

const emit = defineEmits<{ 'update:modelValue': [value: string] }>()
</script>

<template>
  <textarea
    :id="id"
    :rows="rows ?? 3"
    :value="modelValue"
    :placeholder="placeholder"
    :disabled="disabled"
    :class="cn(
      'w-full rounded-lg border bg-canvas px-3 py-2 text-sm text-ink shadow-soft placeholder:text-faint',
      'focus:border-accent focus:outline-none focus:ring-2 focus:ring-accent/30',
      invalid ? 'border-[var(--danger)]' : 'border-line',
    )"
    @input="emit('update:modelValue', ($event.target as HTMLTextAreaElement).value)"
  />
</template>
