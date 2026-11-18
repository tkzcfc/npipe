<script setup lang="ts">
import { computed, ref } from 'vue'
import { cn } from '@/utils/cn'
import Icon from './Icon.vue'

const props = withDefaults(defineProps<{
  modelValue?: string | number
  type?: string
  placeholder?: string
  disabled?: boolean
  invalid?: boolean
  id?: string
  autocomplete?: string
  name?: string
  min?: number | string
  max?: number | string
  step?: number | string
}>(), { type: 'text' })

const emit = defineEmits<{
  'update:modelValue': [value: string]
  keyup: [event: KeyboardEvent]
}>()

const reveal = ref(false)
const isPassword = computed(() => props.type === 'password')
const actualType = computed(() => (isPassword.value ? (reveal.value ? 'text' : 'password') : props.type))
</script>

<template>
  <div class="relative">
    <input
      :id="id"
      :name="name"
      :type="actualType"
      :value="modelValue"
      :placeholder="placeholder"
      :disabled="disabled"
      :autocomplete="autocomplete"
      :min="min"
      :max="max"
      :step="step"
      :class="cn(
        'h-10 w-full rounded-lg border bg-canvas px-3 text-sm text-ink shadow-soft placeholder:text-faint',
        'focus:border-accent focus:outline-none focus:ring-2 focus:ring-accent/30',
        'disabled:cursor-not-allowed disabled:opacity-60',
        isPassword && 'pr-10',
        invalid ? 'border-[var(--danger)]' : 'border-line',
      )"
      @input="emit('update:modelValue', ($event.target as HTMLInputElement).value)"
      @keyup="emit('keyup', $event)"
    />
    <button
      v-if="isPassword"
      type="button"
      class="absolute right-2 top-1/2 -translate-y-1/2 rounded p-1 text-faint hover:text-ink"
      :aria-label="reveal ? 'Hide password' : 'Show password'"
      @click="reveal = !reveal"
    >
      <Icon :name="reveal ? 'eyeOff' : 'eye'" :size="16" />
    </button>
  </div>
</template>
