<template>
  <div class="page-container">
    <div class="page-head">
      <div>
        <h1>{{ $t('maintenance.title') }}</h1>
        <p>{{ $t('maintenance.subtitle') }}</p>
      </div>
      <div class="maintenance-actions">
        <el-button :icon="Refresh" :loading="maintenanceLoading" @click="loadMaintenanceInfo">
          {{ $t('maintenance.refreshInfo') }}
        </el-button>
        <el-button type="danger" :icon="Delete" @click="cleanupDatabase">
          {{ $t('maintenance.cleanup') }}
        </el-button>
      </div>
    </div>

    <section class="panel">
      <el-form class="cleanup-form" :model="cleanupForm" label-position="top">
        <el-form-item :label="$t('maintenance.loginHistoryKeepDays')">
          <el-input-number
            v-model="cleanupForm.login_history_keep_days"
            :min="1"
            :max="3650"
            controls-position="right"
            @change="loadMaintenanceInfo"
          />
        </el-form-item>
        <el-form-item :label="$t('maintenance.operationLogKeepDays')">
          <el-input-number
            v-model="cleanupForm.operation_log_keep_days"
            :min="1"
            :max="3650"
            controls-position="right"
            @change="loadMaintenanceInfo"
          />
        </el-form-item>
        <el-form-item :label="$t('maintenance.trafficHourlyKeepDays')">
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
        <el-table-column prop="name" :label="$t('maintenance.dataType')" min-width="120" />
        <el-table-column prop="total" :label="$t('maintenance.totalCount')" width="130" align="right" />
        <el-table-column :label="$t('maintenance.cleanupCount')" width="150" align="right">
          <template #default="{ row }">
            <el-tag :type="row.cleanup > 0 ? 'warning' : 'success'" size="small">{{ row.cleanup }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="$t('maintenance.oldest')" min-width="170">
          <template #default="{ row }">
            <span>{{ row.oldest || '-' }}</span>
          </template>
        </el-table-column>
        <el-table-column :label="$t('maintenance.newest')" min-width="170">
          <template #default="{ row }">
            <span>{{ row.newest || '-' }}</span>
          </template>
        </el-table-column>
      </el-table>

      <div v-if="cleanupResult" class="cleanup-result">
        <span>{{ $t('maintenance.cleaned') }}</span>
        <el-tag type="info">{{ $t('maintenance.loginHistory') }} {{ cleanupResult.login_history_deleted }}</el-tag>
        <el-tag type="info">{{ $t('maintenance.operationLog') }} {{ cleanupResult.operation_log_deleted }}</el-tag>
        <el-tag type="info">{{ $t('maintenance.trafficHourly') }} {{ cleanupResult.traffic_hourly_deleted }}</el-tag>
      </div>
    </section>

    <ConfirmDelete
      v-model:visible="cleanupDialog.visible"
      :title="$t('maintenance.confirmTitle')"
      :message="$t('maintenance.confirm')"
      :loading="cleanupDialog.loading"
      :confirm-text="$t('maintenance.cleanup')"
      :cancel-text="$t('common.cancel')"
      @confirm="handleCleanupConfirm"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Delete, Refresh } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import { operationApi } from '@/api'
import ConfirmDelete from '@/components/ConfirmDelete.vue'
import type { CleanupDatabaseResponse, DatabaseMaintenanceInfoResponse } from '@/types'

const { t } = useI18n()

const maintenanceLoading = ref(false)
const maintenanceInfo = ref<DatabaseMaintenanceInfoResponse | null>(null)
const cleanupResult = ref<CleanupDatabaseResponse | null>(null)

const cleanupDialog = reactive({
  visible: false,
  loading: false,
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
      name: t('maintenance.loginHistory'),
      total: info?.login_history.total_count ?? 0,
      cleanup: info?.login_history.cleanup_count ?? 0,
      oldest: info?.login_history.oldest ?? '',
      newest: info?.login_history.newest ?? '',
    },
    {
      name: t('maintenance.operationLog'),
      total: info?.operation_log.total_count ?? 0,
      cleanup: info?.operation_log.cleanup_count ?? 0,
      oldest: info?.operation_log.oldest ?? '',
      newest: info?.operation_log.newest ?? '',
    },
    {
      name: t('maintenance.trafficHourly'),
      total: info?.traffic_hourly.total_count ?? 0,
      cleanup: info?.traffic_hourly.cleanup_count ?? 0,
      oldest: info?.traffic_hourly.oldest ?? '',
      newest: info?.traffic_hourly.newest ?? '',
    },
  ]
})

async function loadMaintenanceInfo() {
  maintenanceLoading.value = true
  try {
    const res = await operationApi.maintenanceInfo({ ...cleanupForm })
    maintenanceInfo.value = res.data
  } finally {
    maintenanceLoading.value = false
  }
}

function cleanupDatabase() {
  cleanupDialog.visible = true
}

async function handleCleanupConfirm() {
  cleanupDialog.loading = true
  try {
    const res = await operationApi.cleanupDatabase({ ...cleanupForm })
    cleanupResult.value = res.data
    ElMessage.success(t('maintenance.cleanupSuccess'))
    cleanupDialog.visible = false
    loadMaintenanceInfo()
  } finally {
    cleanupDialog.loading = false
  }
}

onMounted(() => loadMaintenanceInfo())
</script>

<style scoped lang="scss">
.maintenance-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  justify-content: flex-end;
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

@media (max-width: 900px) {
  .cleanup-form {
    grid-template-columns: 1fr;
  }
}
</style>
