<template>
  <div class="page-container">
    <div class="page-head">
      <div>
        <h1>{{ $t('loginLog.title') }}</h1>
        <p>{{ $t('loginLog.subtitle') }}</p>
      </div>
      <el-button :icon="Refresh" @click="loadData(1)" :loading="loading">{{ $t('common.refresh') }}</el-button>
    </div>

    <section class="panel">
      <div class="table-toolbar">
        <div class="filter-row">
          <el-input-number
            v-model="filterUserId"
            :min="1"
            :placeholder="$t('loginLog.userFilter')"
            controls-position="right"
            clearable
          />
          <el-button type="primary" :icon="Search" @click="loadData(1)">{{ $t('common.search') }}</el-button>
          <el-button text @click="clearFilter">{{ $t('common.all') }}</el-button>
        </div>
      </div>

      <el-table v-loading="loading" :data="items" row-key="id" style="width:100%; margin-top: 14px;">
        <el-table-column prop="id" label="ID" width="90" />
        <el-table-column prop="user_id" :label="$t('loginLog.userId')" width="100" />
        <el-table-column prop="ip_addr" label="IP" min-width="150">
          <template #default="{ row }">
            <span class="font-mono">{{ row.ip_addr }}</span>
          </template>
        </el-table-column>
        <el-table-column :label="$t('loginLog.loginTime')" min-width="170">
          <template #default="{ row }">
            <span>{{ row.login_time ? formatTime(row.login_time) : '-' }}</span>
          </template>
        </el-table-column>
        <el-table-column :label="$t('loginLog.logoutTime')" min-width="170">
          <template #default="{ row }">
            <span v-if="row.logout_time">{{ formatTime(row.logout_time) }}</span>
            <el-tag v-else type="success" size="small">{{ $t('loginLog.online') }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="$t('loginLog.duration')" width="130" align="right">
          <template #default="{ row }">
            <span>{{ row.logout_time ? formatDuration(row.duration_secs) : '-' }}</span>
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
import { Refresh, Search } from '@element-plus/icons-vue'
import { playerApi } from '@/api'
import type { LoginHistoryItem } from '@/types'

const loading = ref(false)
const items = ref<LoginHistoryItem[]>([])
const filterUserId = ref<number | undefined>()

const pagination = reactive({
  currentPage: 1,
  pageSize: 20,
  total: 0,
})

async function loadData(page = pagination.currentPage) {
  loading.value = true
  pagination.currentPage = page
  try {
    const res = await playerApi.loginHistory({
      user_id: filterUserId.value,
      page_number: page - 1,
      page_size: pagination.pageSize,
    })
    items.value = res.data.items ?? []
    pagination.total = res.data.total_count ?? 0
  } finally {
    loading.value = false
  }
}

function clearFilter() {
  filterUserId.value = undefined
  loadData(1)
}

function formatTime(ts: number): string {
  const d = new Date(ts * 1000)
  const pad = (n: number) => n.toString().padStart(2, '0')
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
}

function formatDuration(secs: number): string {
  if (secs < 60) return `${secs}s`
  if (secs < 3600) return `${Math.floor(secs / 60)}m ${secs % 60}s`
  const h = Math.floor(secs / 3600)
  const m = Math.floor((secs % 3600) / 60)
  return `${h}h ${m}m`
}

onMounted(() => loadData(1))
</script>

<style scoped lang="scss">
.filter-row {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}

.pager-row {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
}
</style>
