<script setup lang="ts">
import { computed } from 'vue'
import { cn } from '@/utils/cn'

const props = defineProps<{
  modelValue: boolean
  disabled?: boolean
  label?: string
}>()

const emit = defineEmits<{ 'update:modelValue': [value: boolean] }>()
const id = computed(() => `sw-${Math.random().toString(36).slice(2, 8)}`)
</script>

<template>
  <label :for="id" class="inline-flex cursor-pointer items-center gap-2">
    <button
      :id="id"
      type="button"
      role="switch"
      :aria-checked="modelValue"
      :disabled="disabled"
      :class="cn(
        'relative h-5 w-9 rounded-full transition-colors focus-visible:outline-none',
        modelValue ? 'bg-accent' : 'bg-line',
        disabled && 'opacity-50',
      )"
      @click="!disabled && emit('update:modelValue', !modelValue)"
    >
      <span
        :class="cn(
          'absolute top-0.5 left-0.5 h-4 w-4 rounded-full bg-white shadow-soft transition-transform',
          modelValue && 'translate-x-4',
        )"
      />
    </button>
    <span v-if="label" class="text-sm text-ink">{{ label }}</span>
  </label>
</template>
