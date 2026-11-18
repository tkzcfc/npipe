<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { operationApi } from '@/api'
import type { CleanupDatabaseResponse, DatabaseMaintenanceInfoResponse } from '@/types'
import { formatDateTime } from '@/utils/time'
import PageHeader from '@/components/PageHeader.vue'
import SystemTabs from '@/components/SystemTabs.vue'
import Button from '@/components/ui/Button.vue'
import Field from '@/components/ui/Field.vue'
import Input from '@/components/ui/Input.vue'
import ConfirmDialog from '@/components/ui/ConfirmDialog.vue'
import EmptyState from '@/components/ui/EmptyState.vue'
import Spinner from '@/components/ui/Spinner.vue'
import { useToastStore } from '@/stores/toast'

const { t } = useI18n()
const toast = useToastStore()

const loading = ref(false)
const loadError = ref('')
const info = ref<DatabaseMaintenanceInfoResponse | null>(null)
const lastResult = ref<CleanupDatabaseResponse | null>(null)
const confirmOpen = ref(false)
const cleaning = ref(false)

const cleanupForm = reactive({
  login_history_keep_days: 90,
  operation_log_keep_days: 180,
  traffic_hourly_keep_days: 90,
})

const rows = computed(() => {
  const data = info.value
  return [
    {
      key: 'login',
      name: t('maintenance.loginHistory'),
      keep: 'login_history_keep_days' as const,
      total: data?.login_history.total_count ?? 0,
      cleanup: data?.login_history.cleanup_count ?? 0,
      oldest: data?.login_history.oldest ?? '',
      newest: data?.login_history.newest ?? '',
    },
    {
      key: 'ops',
      name: t('maintenance.operationLog'),
      keep: 'operation_log_keep_days' as const,
      total: data?.operation_log.total_count ?? 0,
      cleanup: data?.operation_log.cleanup_count ?? 0,
      oldest: data?.operation_log.oldest ?? '',
      newest: data?.operation_log.newest ?? '',
    },
    {
      key: 'traffic',
      name: t('maintenance.trafficHourly'),
      keep: 'traffic_hourly_keep_days' as const,
      total: data?.traffic_hourly.total_count ?? 0,
      cleanup: data?.traffic_hourly.cleanup_count ?? 0,
      oldest: data?.traffic_hourly.oldest ?? '',
      newest: data?.traffic_hourly.newest ?? '',
    },
  ]
})

async function loadInfo() {
  loading.value = true
  loadError.value = ''
  try {
    const res = await operationApi.maintenanceInfo({ ...cleanupForm })
    info.value = res.data
  } catch {
    loadError.value = t('common.loadFailed')
  } finally {
    loading.value = false
  }
}

let debounce: number | undefined
watch(cleanupForm, () => {
  window.clearTimeout(debounce)
  debounce = window.setTimeout(() => loadInfo(), 400)
}, { deep: true })

async function handleCleanup() {
  cleaning.value = true
  try {
    const res = await operationApi.cleanupDatabase({ ...cleanupForm })
    lastResult.value = res.data
    toast.success(t('maintenance.cleanupSuccess'))
    confirmOpen.value = false
    await loadInfo()
  } finally {
    cleaning.value = false
  }
}

onMounted(() => loadInfo())
</script>

<template>
  <div>
    <SystemTabs />
    <PageHeader :title="t('maintenance.title')" :subtitle="t('maintenance.subtitle')">
      <Button icon="refresh" :loading="loading" @click="loadInfo">{{ t('maintenance.refreshInfo') }}</Button>
      <Button variant="danger" @click="confirmOpen = true">{{ t('maintenance.cleanup') }}</Button>
    </PageHeader>

    <p class="mb-4 text-sm text-muted">{{ t('maintenance.previewHint') }}</p>

    <Spinner v-if="loading && !info" :label="t('common.loading')" />
    <EmptyState v-else-if="loadError" :title="t('common.loadFailed')" :description="loadError" icon="warning" retry @retry="loadInfo" />

    <div v-else class="grid gap-4 xl:grid-cols-3">
      <section v-for="row in rows" :key="row.key" class="ui-card p-5">
        <h2 class="text-sm font-semibold">{{ row.name }}</h2>
        <div class="mt-4">
          <Field :label="t('maintenance.keepDays')">
            <Input
              :model-value="cleanupForm[row.keep]"
              type="number"
              min="1"
              @update:model-value="cleanupForm[row.keep] = Math.max(1, Number($event) || 1)"
            />
          </Field>
        </div>
        <dl class="mt-4 space-y-2 text-sm">
          <div class="flex justify-between"><dt class="text-muted">{{ t('maintenance.totalCount') }}</dt><dd class="tabular">{{ row.total }}</dd></div>
          <div class="flex justify-between"><dt class="text-muted">{{ t('maintenance.cleanupCount') }}</dt><dd class="tabular text-[var(--danger)]">{{ row.cleanup }}</dd></div>
          <div class="flex justify-between gap-3"><dt class="text-muted">{{ t('maintenance.oldest') }}</dt><dd class="tabular text-right">{{ row.oldest ? formatDateTime(row.oldest) : t('maintenance.emptyRange') }}</dd></div>
          <div class="flex justify-between gap-3"><dt class="text-muted">{{ t('maintenance.newest') }}</dt><dd class="tabular text-right">{{ row.newest ? formatDateTime(row.newest) : t('maintenance.emptyRange') }}</dd></div>
        </dl>
      </section>
    </div>

    <section v-if="lastResult" class="ui-card mt-4 p-5">
      <h2 class="text-sm font-semibold">{{ t('maintenance.lastResult') }}</h2>
      <dl class="mt-3 grid gap-2 text-sm sm:grid-cols-3">
        <div><dt class="text-muted">{{ t('maintenance.loginHistory') }}</dt><dd class="tabular">{{ lastResult.login_history_deleted }}</dd></div>
        <div><dt class="text-muted">{{ t('maintenance.operationLog') }}</dt><dd class="tabular">{{ lastResult.operation_log_deleted }}</dd></div>
        <div><dt class="text-muted">{{ t('maintenance.trafficHourly') }}</dt><dd class="tabular">{{ lastResult.traffic_hourly_deleted }}</dd></div>
      </dl>
    </section>

    <ConfirmDialog
      :open="confirmOpen"
      :title="t('maintenance.confirmTitle')"
      :message="t('maintenance.confirm')"
      :loading="cleaning"
      :confirm-text="t('maintenance.cleanup')"
      tone="danger"
      @update:open="confirmOpen = $event"
      @confirm="handleCleanup"
    />
  </div>
</template>
