<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import Button from './Button.vue'
import Select from './Select.vue'

const props = withDefaults(defineProps<{
  page: number
  pageSize: number
  total: number
  pageSizes?: number[]
}>(), { pageSizes: () => [20, 50, 100] })

const emit = defineEmits<{
  'update:page': [value: number]
  'update:pageSize': [value: number]
}>()

const { t } = useI18n()
const pages = computed(() => Math.max(1, Math.ceil(props.total / props.pageSize)))
</script>

<template>
  <div class="flex flex-wrap items-center justify-between gap-3 px-1 py-3 text-sm text-muted">
    <span class="tabular">{{ t('common.totalItems', { total }) }}</span>
    <div class="flex items-center gap-2">
      <Select
        :model-value="pageSize"
        class="!h-8 !w-[5.5rem]"
        @update:model-value="emit('update:pageSize', Number($event)); emit('update:page', 1)"
      >
        <option v-for="size in pageSizes" :key="size" :value="size">{{ size }}</option>
      </Select>
      <Button variant="secondary" size="sm" icon="chevronLeft" :disabled="page <= 1" @click="emit('update:page', page - 1)" />
      <span class="tabular min-w-[4rem] text-center">{{ page }} / {{ pages }}</span>
      <Button variant="secondary" size="sm" icon="chevronRight" :disabled="page >= pages" @click="emit('update:page', page + 1)" />
    </div>
  </div>
</template>
