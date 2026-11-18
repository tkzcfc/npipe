<script setup lang="ts">
import { cn } from '@/utils/cn'

defineProps<{
  modelValue: string | number
  options: { label: string; value: string | number }[]
}>()

const emit = defineEmits<{ 'update:modelValue': [value: string | number] }>()
</script>

<template>
  <div role="tablist" class="inline-flex flex-wrap rounded-lg border border-line bg-canvas p-0.5">
    <button
      v-for="opt in options"
      :key="String(opt.value)"
      type="button"
      role="tab"
      :aria-selected="modelValue === opt.value"
      :class="cn(
        'h-8 rounded-md px-3 text-[13px] font-medium transition-colors',
        modelValue === opt.value ? 'bg-surface text-ink shadow-soft' : 'text-muted hover:text-ink',
      )"
      @click="emit('update:modelValue', opt.value)"
    >
      {{ opt.label }}
    </button>
  </div>
</template>
