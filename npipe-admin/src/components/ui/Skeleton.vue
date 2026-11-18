<script setup lang="ts">
import { cn } from '@/utils/cn'
defineProps<{ class?: string }>()
</script>

<template>
  <div :class="cn('animate-pulse rounded-md bg-line', $props.class)" />
</template>
