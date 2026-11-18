<script setup lang="ts">
defineProps<{ label?: string }>()
</script>

<template>
  <div class="flex items-center justify-center gap-2 py-10 text-sm text-muted" role="status">
    <span class="h-4 w-4 animate-spin rounded-full border-2 border-accent border-t-transparent" />
    <span>{{ label }}</span>
  </div>
</template>
