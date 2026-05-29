<template>
  <div class="page-container">
    <div class="page-head">
      <div>
        <h1>{{ $t('operationLog.title') }}</h1>
        <p>{{ $t('operationLog.subtitle') }}</p>
      </div>
      <el-button :icon="Refresh" :loading="loading" @click="loadData(1)">{{ $t('common.refresh') }}</el-button>
    </div>

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
import { onMounted, reactive, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Refresh } from '@element-plus/icons-vue'
import { operationApi } from '@/api'
import type { OperationLogItem } from '@/types'

const { t } = useI18n()

const loading = ref(false)
const items = ref<OperationLogItem[]>([])

const pagination = reactive({
  currentPage: 1,
  pageSize: 20,
  total: 0,
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

function actionLabel(action: string): string {
  return t(`operationLog.actions.${action}`)
}

function targetLabel(target: string): string {
  return t(`operationLog.targets.${target}`)
}

onMounted(() => loadData(1))
</script>

<style scoped lang="scss">
.pager-row {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
}
</style>
