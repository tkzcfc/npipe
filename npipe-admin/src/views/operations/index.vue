<template>
  <div class="page-container">
    <div class="page-head">
      <div>
        <h1>{{ $t('operationLog.title') }}</h1>
        <p>{{ $t('operationLog.subtitle') }}</p>
      </div>
      <el-button :icon="Refresh" :loading="loading" @click="loadData(1)">{{ $t('common.refresh') }}</el-button>
    </div>

    <section class="panel maintenance-panel">
      <div class="panel-head">
        <div>
          <h2>{{ $t('operationLog.maintenance.title') }}</h2>
          <p>{{ $t('operationLog.maintenance.subtitle') }}</p>
        </div>
        <div class="maintenance-actions">
          <el-button :icon="Refresh" :loading="maintenanceLoading" @click="loadMaintenanceInfo">
            {{ $t('operationLog.maintenance.refreshInfo') }}
          </el-button>
          <el-button type="danger" :icon="Delete" :loading="cleanupLoading" @click="cleanupDatabase">
            {{ $t('operationLog.maintenance.cleanup') }}
          </el-button>
        </div>
      </div>

      <el-form class="cleanup-form" :model="cleanupForm" label-position="top">
        <el-form-item :label="$t('operationLog.maintenance.loginHistoryKeepDays')">
          <el-input-number
            v-model="cleanupForm.login_history_keep_days"
            :min="1"
            :max="3650"
            controls-position="right"
            @change="loadMaintenanceInfo"
          />
        </el-form-item>
        <el-form-item :label="$t('operationLog.maintenance.operationLogKeepDays')">
          <el-input-number
            v-model="cleanupForm.operation_log_keep_days"
            :min="1"
            :max="3650"
            controls-position="right"
            @change="loadMaintenanceInfo"
          />
        </el-form-item>
        <el-form-item :label="$t('operationLog.maintenance.trafficHourlyKeepDays')">
          <el-input-number
            v-model="cleanupForm.traffic_hourly_keep_days"
            :min="1"
            :max="3650"
            controls-position="right"
            @change="loadMaintenanceInfo"
          />
        </el-form-item>
      </el-form>

      <el-table
        v-loading="maintenanceLoading"
        :data="maintenanceRows"
        class="maintenance-table"
        size="small"
        style="width:100%;"
      >
        <el-table-column prop="name" :label="$t('operationLog.maintenance.dataType')" min-width="120" />
        <el-table-column prop="total" :label="$t('operationLog.maintenance.totalCount')" width="130" align="right" />
        <el-table-column :label="$t('operationLog.maintenance.cleanupCount')" width="150" align="right">
          <template #default="{ row }">
            <el-tag :type="row.cleanup > 0 ? 'warning' : 'success'" size="small">{{ row.cleanup }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="$t('operationLog.maintenance.oldest')" min-width="170">
          <template #default="{ row }">
            <span>{{ row.oldest || '-' }}</span>
          </template>
        </el-table-column>
        <el-table-column :label="$t('operationLog.maintenance.newest')" min-width="170">
          <template #default="{ row }">
            <span>{{ row.newest || '-' }}</span>
          </template>
        </el-table-column>
      </el-table>

      <div v-if="cleanupResult" class="cleanup-result">
        <span>{{ $t('operationLog.maintenance.cleaned') }}</span>
        <el-tag type="info">{{ $t('operationLog.maintenance.loginHistory') }} {{ cleanupResult.login_history_deleted }}</el-tag>
        <el-tag type="info">{{ $t('operationLog.maintenance.operationLog') }} {{ cleanupResult.operation_log_deleted }}</el-tag>
        <el-tag type="info">{{ $t('operationLog.maintenance.trafficHourly') }} {{ cleanupResult.traffic_hourly_deleted }}</el-tag>
      </div>
    </section>

    <section class="panel">
      <el-table v-loading="loading" :data="items" row-key="id" style="width:100%;">
        <el-table-column prop="id" label="ID" width="90" />
        <el-table-column prop="created_at" :label="$t('operationLog.time')" min-width="170" />
        <el-table-column prop="actor" :label="$t('operationLog.actor')" width="100" />
        <el-table-column :label="$t('operationLog.action')" min-width="170">
          <template #default="{ row }">
            <el-tag size="small" type="primary">{{ actionLabel(row.action) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="$t('operationLog.target')" min-width="180">
          <template #default="{ row }">
            <span>{{ targetLabel(row.target_type) }}</span>
            <span v-if="row.target_id" class="font-mono"> #{{ row.target_id }}</span>
            <span v-if="row.target_name"> · {{ row.target_name }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="detail" :label="$t('operationLog.detail')" min-width="160" show-overflow-tooltip>
          <template #default="{ row }">
            <span v-if="row.detail">{{ row.detail }}</span>
            <span v-else class="text-muted">-</span>
          </template>
        </el-table-column>
      </el-table>

      <div class="pager-row">
        <el-pagination
          v-model:current-page="pagination.currentPage"
          v-model:page-size="pagination.pageSize"
          :total="pagination.total"
          layout="total, prev, pager, next"
          background
          @current-change="loadData"
        />
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Delete, Refresh } from '@element-plus/icons-vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { operationApi } from '@/api'
import type { CleanupDatabaseResponse, DatabaseMaintenanceInfoResponse, OperationLogItem } from '@/types'

const { t } = useI18n()

const loading = ref(false)
const maintenanceLoading = ref(false)
const cleanupLoading = ref(false)
const items = ref<OperationLogItem[]>([])
const maintenanceInfo = ref<DatabaseMaintenanceInfoResponse | null>(null)
const cleanupResult = ref<CleanupDatabaseResponse | null>(null)

const pagination = reactive({
  currentPage: 1,
  pageSize: 20,
  total: 0,
})

const cleanupForm = reactive({
  login_history_keep_days: 90,
  operation_log_keep_days: 180,
  traffic_hourly_keep_days: 90,
})

const maintenanceRows = computed(() => {
  const info = maintenanceInfo.value
  return [
    {
      name: t('operationLog.maintenance.loginHistory'),
      total: info?.login_history.total_count ?? 0,
      cleanup: info?.login_history.cleanup_count ?? 0,
      oldest: info?.login_history.oldest ?? '',
      newest: info?.login_history.newest ?? '',
    },
    {
      name: t('operationLog.maintenance.operationLog'),
      total: info?.operation_log.total_count ?? 0,
      cleanup: info?.operation_log.cleanup_count ?? 0,
      oldest: info?.operation_log.oldest ?? '',
      newest: info?.operation_log.newest ?? '',
    },
    {
      name: t('operationLog.maintenance.trafficHourly'),
      total: info?.traffic_hourly.total_count ?? 0,
      cleanup: info?.traffic_hourly.cleanup_count ?? 0,
      oldest: info?.traffic_hourly.oldest ?? '',
      newest: info?.traffic_hourly.newest ?? '',
    },
  ]
})

async function loadData(page = pagination.currentPage) {
  loading.value = true
  pagination.currentPage = page
  try {
    const res = await operationApi.logs({
      page_number: page - 1,
      page_size: pagination.pageSize,
    })
    items.value = res.data.items ?? []
    pagination.total = res.data.total_count ?? 0
  } finally {
    loading.value = false
  }
}

async function loadMaintenanceInfo() {
  maintenanceLoading.value = true
  try {
    const res = await operationApi.maintenanceInfo({ ...cleanupForm })
    maintenanceInfo.value = res.data
  } finally {
    maintenanceLoading.value = false
  }
}

async function cleanupDatabase() {
  await ElMessageBox.confirm(
    t('operationLog.maintenance.confirm'),
    t('operationLog.maintenance.confirmTitle'),
    {
      confirmButtonText: t('operationLog.maintenance.cleanup'),
      cancelButtonText: t('common.cancel'),
      type: 'warning',
      confirmButtonClass: 'el-button--danger',
    },
  )

  cleanupLoading.value = true
  try {
    const res = await operationApi.cleanupDatabase({ ...cleanupForm })
    cleanupResult.value = res.data
    ElMessage.success(t('operationLog.maintenance.cleanupSuccess'))
    loadMaintenanceInfo()
    loadData(1)
  } finally {
    cleanupLoading.value = false
  }
}

function actionLabel(action: string): string {
  return t(`operationLog.actions.${action}`)
}

function targetLabel(target: string): string {
  return t(`operationLog.targets.${target}`)
}

onMounted(() => {
  loadMaintenanceInfo()
  loadData(1)
})
</script>

<style scoped lang="scss">
.maintenance-panel {
  margin-bottom: 16px;
}

.maintenance-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  justify-content: flex-end;
}

.panel-head p {
  margin: 4px 0 0;
  font-size: 13px;
  color: var(--text-muted);
}

.cleanup-form {
  display: grid;
  grid-template-columns: repeat(3, minmax(180px, 1fr));
  gap: 12px;

  :deep(.el-form-item) {
    margin-bottom: 0;
  }

  :deep(.el-input-number) {
    width: 100%;
  }
}

.maintenance-table {
  margin-top: 14px;
}

.cleanup-result {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 14px;
  color: var(--text-secondary);
  font-size: 13px;
}

.pager-row {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
}

@media (max-width: 900px) {
  .cleanup-form {
    grid-template-columns: 1fr;
  }
}
</style>
