<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute } from 'vue-router'
import { playerApi } from '@/api'
import { useAuthStore } from '@/stores/auth'
import type { LoginHistoryItem } from '@/types'
import { formatDuration } from '@/utils/format'
import { formatDateTime } from '@/utils/time'
import { accountScope, loadFilters, saveFilters } from '@/utils/filters'
import PageHeader from '@/components/PageHeader.vue'
import SystemTabs from '@/components/SystemTabs.vue'
import StatusBadge from '@/components/StatusBadge.vue'
import Button from '@/components/ui/Button.vue'
import Input from '@/components/ui/Input.vue'
import EmptyState from '@/components/ui/EmptyState.vue'
import Spinner from '@/components/ui/Spinner.vue'
import Pagination from '@/components/ui/Pagination.vue'

const { t } = useI18n()
const route = useRoute()
const authStore = useAuthStore()

const loading = ref(false)
const loadError = ref('')
const items = ref<LoginHistoryItem[]>([])
const filterUserId = ref('')
const pagination = reactive({ currentPage: 1, pageSize: 20, total: 0 })

const parsedUserId = computed(() => {
  const raw = filterUserId.value.trim()
  if (!raw) return undefined
  const n = Number(raw)
  return Number.isFinite(n) ? n : undefined
})

async function loadData(page = pagination.currentPage) {
  loading.value = true
  loadError.value = ''
  pagination.currentPage = page
  try {
    const res = await playerApi.loginHistory({
      user_id: authStore.isAdmin ? parsedUserId.value : undefined,
      page_number: page - 1,
      page_size: pagination.pageSize,
    })
    items.value = res.data.items ?? []
    pagination.total = res.data.total_count ?? 0
  } catch {
    loadError.value = t('common.loadFailed')
  } finally {
    loading.value = false
  }
}

function clearFilter() {
  filterUserId.value = ''
  loadData(1)
}

function sourceLabel(source: string) {
  return source === 'web' ? t('loginLog.sourceWeb') : t('loginLog.sourceClient')
}

let persistReady = false

function restoreFilters() {
  const queryId = String(route.query.user_id ?? '').trim()
  if (queryId) {
    filterUserId.value = queryId
    return
  }
  const saved = loadFilters<{ userId?: string }>('logins', accountScope(authStore.role, authStore.currentUserId))
  if (saved?.userId && authStore.isAdmin) filterUserId.value = saved.userId
}

watch([filterUserId], () => {
  if (!persistReady || !authStore.isAdmin) return
  saveFilters('logins', accountScope(authStore.role, authStore.currentUserId), { userId: filterUserId.value })
})

watch(() => route.query.user_id, (value) => {
  const next = String(value ?? '').trim()
  if (next && next !== filterUserId.value) {
    filterUserId.value = next
    loadData(1)
  }
})

onMounted(() => {
  restoreFilters()
  persistReady = true
  loadData(1)
})
</script>

<template>
  <div>
    <SystemTabs />
    <PageHeader :title="authStore.isAdmin ? t('loginLog.title') : t('layout.myLoginLogs')" :subtitle="authStore.isAdmin ? t('loginLog.subtitle') : ''">
      <Button icon="refresh" :loading="loading" @click="loadData(pagination.currentPage)">{{ t('common.refresh') }}</Button>
    </PageHeader>

    <div class="ui-card overflow-hidden">
      <div v-if="authStore.isAdmin" class="flex flex-wrap items-end gap-2 border-b border-line px-4 py-3">
        <div class="w-40">
          <Input v-model="filterUserId" type="number" min="0" :placeholder="t('loginLog.userFilterPlaceholder')" />
        </div>
        <Button size="sm" variant="primary" @click="loadData(1)">{{ t('common.search') }}</Button>
        <Button size="sm" variant="ghost" @click="clearFilter">{{ t('common.all') }}</Button>
      </div>

      <Spinner v-if="loading && !items.length" :label="t('common.loading')" />
      <EmptyState v-else-if="loadError" :title="t('common.loadFailed')" :description="loadError" icon="warning" retry @retry="loadData()" />
      <EmptyState v-else-if="!items.length" :title="t('loginLog.empty')" icon="log" />

      <div v-else class="overflow-x-auto">
        <table class="min-w-full text-left text-sm">
          <thead class="border-b border-line bg-canvas text-xs uppercase tracking-wide text-faint">
            <tr>
              <th class="px-4 py-2 font-medium">{{ t('loginLog.userId') }}</th>
              <th class="px-4 py-2 font-medium">{{ t('loginLog.ip') }}</th>
              <th class="px-4 py-2 font-medium">{{ t('loginLog.source') }}</th>
              <th class="px-4 py-2 font-medium">{{ t('loginLog.result') }}</th>
              <th class="px-4 py-2 font-medium">{{ t('loginLog.loginTime') }}</th>
              <th class="px-4 py-2 font-medium">{{ t('loginLog.logoutTime') }}</th>
              <th class="px-4 py-2 font-medium">{{ t('loginLog.duration') }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="item in items" :key="item.id" class="border-b border-line last:border-0">
              <td class="px-4 py-2 tabular">{{ item.user_id }}</td>
              <td class="mono px-4 py-2 text-[13px]">{{ item.ip_addr || '—' }}</td>
              <td class="px-4 py-2">{{ sourceLabel(item.login_source) }}</td>
              <td class="px-4 py-2">
                <StatusBadge :tone="item.success ? 'success' : 'danger'" :label="item.success ? t('loginLog.success') : t('loginLog.failed')" />
              </td>
              <td class="px-4 py-2 tabular">{{ formatDateTime(item.login_time) }}</td>
              <td class="px-4 py-2 tabular">{{ item.logout_time ? formatDateTime(item.logout_time) : '—' }}</td>
              <td class="px-4 py-2">{{ item.logout_time ? formatDuration(item.duration_secs) : t('loginLog.online') }}</td>
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
