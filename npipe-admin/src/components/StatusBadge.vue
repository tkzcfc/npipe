<script setup lang="ts">
import Badge from './ui/Badge.vue'

defineProps<{
  tone?: 'neutral' | 'success' | 'warning' | 'danger' | 'accent' | 'info'
  pulse?: boolean
  label: string
}>()
</script>

<template>
  <Badge :tone="tone">
    <span
      v-if="pulse"
      class="h-1.5 w-1.5 rounded-full bg-current"
    />
    {{ label }}
  </Badge>
</template>
