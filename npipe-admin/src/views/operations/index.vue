<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { operationApi } from '@/api'
import type { OperationLogItem } from '@/types'
import { formatDateTime } from '@/utils/time'
import PageHeader from '@/components/PageHeader.vue'
import SystemTabs from '@/components/SystemTabs.vue'
import Button from '@/components/ui/Button.vue'
import EmptyState from '@/components/ui/EmptyState.vue'
import Spinner from '@/components/ui/Spinner.vue'
import Pagination from '@/components/ui/Pagination.vue'

const { t } = useI18n()
const loading = ref(false)
const loadError = ref('')
const items = ref<OperationLogItem[]>([])
const pagination = reactive({ currentPage: 1, pageSize: 20, total: 0 })

async function loadData(page = pagination.currentPage) {
  loading.value = true
  loadError.value = ''
  pagination.currentPage = page
  try {
    const res = await operationApi.logs({ page_number: page - 1, page_size: pagination.pageSize })
    items.value = res.data.items ?? []
    pagination.total = res.data.total_count ?? 0
  } catch {
    loadError.value = t('common.loadFailed')
  } finally {
    loading.value = false
  }
}

function actionLabel(action: string) {
  const key = `operationLog.actions.${action}`
  const translated = t(key)
  return translated === key ? action : translated
}
function targetLabel(target: string) {
  const key = `operationLog.targets.${target}`
  const translated = t(key)
  return translated === key ? target : translated
}

onMounted(() => loadData(1))
</script>

<template>
  <div>
    <SystemTabs />
    <PageHeader :title="t('operationLog.title')" :subtitle="t('operationLog.subtitle')">
      <Button icon="refresh" :loading="loading" @click="loadData(pagination.currentPage)">{{ t('common.refresh') }}</Button>
    </PageHeader>

    <div class="ui-card overflow-hidden">
      <Spinner v-if="loading && !items.length" :label="t('common.loading')" />
      <EmptyState v-else-if="loadError" :title="t('common.loadFailed')" :description="loadError" icon="warning" retry @retry="loadData()" />
      <EmptyState v-else-if="!items.length" :title="t('operationLog.empty')" icon="clipboard" />

      <div v-else class="overflow-x-auto">
        <table class="min-w-full text-left text-sm">
          <thead class="border-b border-line bg-canvas text-xs uppercase tracking-wide text-faint">
            <tr>
              <th class="px-4 py-2 font-medium">{{ t('operationLog.time') }}</th>
              <th class="px-4 py-2 font-medium">{{ t('operationLog.actor') }}</th>
              <th class="px-4 py-2 font-medium">{{ t('operationLog.action') }}</th>
              <th class="px-4 py-2 font-medium">{{ t('operationLog.target') }}</th>
              <th class="px-4 py-2 font-medium">{{ t('operationLog.detail') }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="item in items" :key="item.id" class="border-b border-line last:border-0">
              <td class="px-4 py-2 tabular whitespace-nowrap">{{ formatDateTime(item.created_at) }}</td>
              <td class="px-4 py-2">{{ item.actor }}</td>
              <td class="px-4 py-2">{{ actionLabel(item.action) }}</td>
              <td class="px-4 py-2">{{ targetLabel(item.target_type) }} #{{ item.target_id }} {{ item.target_name }}</td>
              <td class="max-w-[28rem] px-4 py-2 text-muted">{{ item.detail || '—' }}</td>
            </tr>
          </tbody>
        </table>
      </div>

      <Pagination
        v-if="pagination.total > 0"
        class="border-t border-line px-4"
        :page="pagination.currentPage"
        :page-size="pagination.pageSize"
        :total="pagination.total"
        @update:page="loadData($event)"
        @update:page-size="pagination.pageSize = $event; loadData(1)"
      />
    </div>
  </div>
</template>
