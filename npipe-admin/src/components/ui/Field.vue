<script setup lang="ts">
defineProps<{
  label?: string
  hint?: string
  error?: string
  required?: boolean
  htmlFor?: string
}>()
</script>

<template>
  <label class="block space-y-1.5" :for="htmlFor">
    <span v-if="label" class="flex items-center gap-1 text-[13px] font-medium text-ink">
      {{ label }}
      <span v-if="required" class="text-[var(--danger)]">*</span>
    </span>
    <slot />
    <p v-if="error" class="text-xs text-[var(--danger)]">{{ error }}</p>
    <p v-else-if="hint" class="text-xs text-faint">{{ hint }}</p>
  </label>
</template>
