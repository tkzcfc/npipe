<script setup lang="ts">
import { computed } from 'vue'
import { iconPaths, type IconName } from '@/components/icons'
import { cn } from '@/utils/cn'

const props = withDefaults(defineProps<{
  name: IconName | string
  size?: number
  class?: string
}>(), { size: 18 })

const html = computed(() => iconPaths[props.name] ?? '')
</script>

<template>
  <svg
    :class="cn('inline-block shrink-0', props.class)"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.75"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
    v-html="html"
  />
</template>
