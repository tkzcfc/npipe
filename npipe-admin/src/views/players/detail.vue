<template>
  <div class="page-container">
    <div class="page-head detail-head">
      <div>
        <el-button v-if="authStore.isAdmin" :icon="ArrowLeft" text class="back-btn" @click="router.push('/players')">
          {{ $t('player.title') }}
        </el-button>
        <h1>{{ player?.username ?? $t('player.detailTitle') }}</h1>
        <p>{{ player ? `ID ${player.id}` : $t('player.detailTitle') }}</p>
      </div>
      <el-button :icon="Refresh" :loading="loading" @click="loadDetail">{{ $t('common.refresh') }}</el-button>
    </div>

    <div v-loading="loading" class="detail-grid">
      <template v-if="player">
        <section class="panel summary-panel">
          <div class="summary-card">
            <span>{{ $t('player.table.status') }}</span>
            <strong>
              <el-tag :type="player.enabled ? player.online ? 'success' : 'info' : 'danger'" size="small">
                {{ player.enabled ? player.online ? $t('common.online') : $t('common.offline') : $t('player.disabled') }}
              </el-tag>
            </strong>
          </div>
          <div class="summary-card">
            <span>{{ $t('player.detailTraffic24h') }}</span>
            <strong>{{ formatBytes(player.traffic_24h_in + player.traffic_24h_out) }}</strong>
          </div>
          <div class="summary-card">
            <span>{{ $t('player.detailTunnels') }}</span>
            <strong>{{ player.tunnels.length }}</strong>
          </div>
        </section>

        <section class="panel action-panel">
          <div class="panel-head">
            <h3>{{ $t('player.accountActions') }}</h3>
          </div>
          <div class="action-grid">
            <el-button v-if="authStore.isAdmin" :icon="Edit" @click="openRenameDialog">{{ $t('player.rename') }}</el-button>
            <el-button :icon="Lock" @click="openPasswordDialog">{{ $t('player.resetPassword') }}</el-button>
            <el-button
              v-if="authStore.isAdmin"
              :icon="player.enabled ? CircleClose : SuccessFilled"
              @click="handleToggleStatus"
            >
              {{ player.enabled ? $t('player.disable') : $t('player.enable') }}
            </el-button>
            <el-button v-if="authStore.isAdmin" :icon="View" @click="handleToggleWebAccess">
              {{ player.web_access ? $t('player.revokeWebAccess') : $t('player.grantWebAccess') }}
            </el-button>
            <el-button v-if="authStore.isAdmin" :icon="SwitchButton" :disabled="!player.online" @click="handleKick">
              {{ $t('player.kick') }}
            </el-button>
            <el-button v-if="authStore.isAdmin" :icon="Delete" type="danger" plain @click="handleRemove">
              {{ $t('player.delete') }}
            </el-button>
          </div>
        </section>

        <section class="panel">
          <div class="panel-head">
            <h3>{{ $t('player.detailTitle') }}</h3>
          </div>
          <el-descriptions :column="2" border size="small">
            <el-descriptions-item :label="$t('common.id')">{{ player.id }}</el-descriptions-item>
            <el-descriptions-item :label="$t('player.username')">{{ player.username }}</el-descriptions-item>
            <el-descriptions-item :label="$t('player.table.account')">
              {{ player.enabled ? $t('player.enabled') : $t('player.disabled') }}
            </el-descriptions-item>
            <el-descriptions-item :label="$t('player.table.ip')">
              <span class="font-mono">{{ player.online ? player.ip_addr : '-' }}</span>
            </el-descriptions-item>
            <el-descriptions-item :label="$t('player.table.protocol')">
              <el-tag v-if="player.online && player.connection_protocol" size="small" effect="plain">
                {{ player.connection_protocol.toUpperCase() }}
              </el-tag>
              <span v-else>-</span>
            </el-descriptions-item>
            <el-descriptions-item :label="$t('player.table.onlineTime')">
              {{ player.online ? formatDuration(player.online_time) : '-' }}
            </el-descriptions-item>
            <el-descriptions-item :label="$t('player.createTime')">{{ player.create_time }}</el-descriptions-item>
            <el-descriptions-item :label="$t('player.currentTraffic')">
              ↓ {{ formatBytes(player.bytes_in) }} / ↑ {{ formatBytes(player.bytes_out) }}
            </el-descriptions-item>
          </el-descriptions>
        </section>

        <section class="panel traffic-panel">
          <div class="panel-head traffic-head">
            <h3>{{ $t('player.trafficTrend') }}</h3>
            <el-radio-group v-model="trafficHours" size="small" @change="loadTrafficStats">
              <el-radio-button v-for="item in trafficRangeOptions" :key="item.value" :value="item.value">
                {{ item.label }}
              </el-radio-button>
            </el-radio-group>
          </div>
          <div v-loading="trafficLoading" class="traffic-chart-wrap" :class="{ 'is-light': appStore.theme === 'light' }">
            <div class="traffic-total">
              <span>{{ $t('player.trafficIn') }} {{ formatBytes(trafficStats?.total_in ?? 0) }}</span>
              <span>{{ $t('player.trafficOut') }} {{ formatBytes(trafficStats?.total_out ?? 0) }}</span>
            </div>
            <VChart class="traffic-chart" :option="trafficChartOption" autoresize />
          </div>
        </section>

        <section class="panel">
          <div class="panel-head">
            <h3>{{ $t('player.associatedTunnels') }}</h3>
          </div>
          <el-table :data="player.tunnels" size="small" empty-text="-">
            <el-table-column prop="id" label="ID" width="72" />
            <el-table-column :label="$t('tunnel.table.source')" min-width="180">
              <template #default="{ row }"><code class="font-mono">{{ row.source }}</code></template>
            </el-table-column>
            <el-table-column :label="$t('tunnel.table.type')" width="100">
              <template #default="{ row }">{{ TUNNEL_TYPE_NAMES[row.tunnel_type] ?? row.tunnel_type }}</template>
            </el-table-column>
            <el-table-column :label="$t('player.role')" width="100">
              <template #default="{ row }">{{ $t(`player.tunnelRole.${row.role}`) }}</template>
            </el-table-column>
            <el-table-column :label="$t('tunnel.table.runtime')" width="120">
              <template #default="{ row }">
                <el-tag :type="row.available ? 'success' : row.enabled ? 'warning' : 'info'" size="small">
                  {{ row.available ? $t('tunnel.runtime.available') : row.enabled ? $t('tunnel.runtime.waiting') : $t('tunnel.runtime.disabled') }}
                </el-tag>
              </template>
            </el-table-column>
          </el-table>
        </section>

        <section class="panel">
          <div class="panel-head">
            <h3>{{ $t('player.recentLogins') }}</h3>
          </div>
          <el-table :data="player.recent_logins" size="small" empty-text="-">
            <el-table-column prop="ip_addr" :label="$t('player.table.ip')" min-width="160" />
            <el-table-column prop="login_time" :label="$t('loginLog.loginTime')" min-width="180" />
            <el-table-column :label="$t('loginLog.duration')" width="130">
              <template #default="{ row }">{{ row.logout_time ? formatDuration(row.duration_secs) : $t('loginLog.online') }}</template>
            </el-table-column>
          </el-table>
        </section>
      </template>

      <el-empty v-else-if="!loading" :description="$t('player.notFound')" />
    </div>

    <el-dialog
      v-model="renameDialog.visible"
      :title="$t('player.renameTitle')"
      width="440px"
      destroy-on-close
    >
      <el-form
        ref="renameFormRef"
        :model="renameDialog.form"
        :rules="renameRules"
        label-width="80px"
        @submit.prevent
      >
        <el-form-item :label="$t('common.id')">
          <el-input :value="renameDialog.form.id" readonly />
        </el-form-item>
        <el-form-item :label="$t('player.username')" prop="username">
          <el-input v-model="renameDialog.form.username" :placeholder="$t('login.usernamePlaceholder')" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="renameDialog.visible = false">{{ $t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="renameDialog.loading" @click="handleRename">{{ $t('common.save') }}</el-button>
      </template>
    </el-dialog>

    <el-dialog
      v-model="passwordDialog.visible"
      :title="$t('player.resetPasswordTitle')"
      width="440px"
      destroy-on-close
    >
      <el-form
        ref="passwordFormRef"
        :model="passwordDialog.form"
        :rules="passwordRules"
        label-width="80px"
        @submit.prevent
      >
        <el-form-item :label="$t('common.id')">
          <el-input :value="passwordDialog.form.id" readonly />
        </el-form-item>
        <el-form-item :label="$t('player.newPassword')" prop="password">
          <el-input v-model="passwordDialog.form.password" type="password" show-password :placeholder="$t('login.passwordPlaceholder')" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="passwordDialog.visible = false">{{ $t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="passwordDialog.loading" @click="handleResetPassword">{{ $t('common.save') }}</el-button>
      </template>
    </el-dialog>

    <ConfirmAction
      v-model:visible="deleteDialog.visible"
      :title="$t('player.deleteTitle')"
      :message="$t('player.deleteConfirm', { name: deleteDialog.username })"
      :details="deleteDialog.details"
      :loading="deleteDialog.loading"
      :confirm-text="$t('player.deleteBtn')"
      :cancel-text="$t('common.cancel')"
      confirm-type="danger"
      :warning-text="$t('common.irreversible')"
      @confirm="handleRemoveConfirm"
    />

    <ConfirmAction
      v-model:visible="actionDialog.visible"
      :title="actionDialog.title"
      :message="actionDialog.message"
      :details="actionDialog.details"
      :loading="actionDialog.loading"
      :confirm-text="actionDialog.confirmText"
      :cancel-text="$t('common.cancel')"
      :confirm-type="actionDialog.confirmType"
      @confirm="handleActionConfirm"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { ArrowLeft, CircleClose, Delete, Edit, Lock, Refresh, SuccessFilled, SwitchButton, View } from '@element-plus/icons-vue'
import VChart from 'vue-echarts'
import { use } from 'echarts/core'
import { CanvasRenderer } from 'echarts/renderers'
import { LineChart } from 'echarts/charts'
import { GridComponent, LegendComponent, TooltipComponent } from 'echarts/components'
import { playerApi } from '@/api'
import { useAuthStore } from '@/stores/auth'
import { useAppStore } from '@/stores/app'
import ConfirmAction from '@/components/ConfirmAction.vue'
import type { PlayerDetail, TrafficStatsResponse } from '@/types'

const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const authStore = useAuthStore()
const appStore = useAppStore()
use([CanvasRenderer, LineChart, GridComponent, LegendComponent, TooltipComponent])

const TUNNEL_TYPE_NAMES: Record<number, string> = { 0: 'TCP', 1: 'UDP', 2: 'SOCKS5', 3: 'HTTP' }
const loading = ref(false)
const trafficLoading = ref(false)
const player = ref<PlayerDetail | null>(null)
const trafficStats = ref<TrafficStatsResponse | null>(null)
const trafficHours = ref(24)
const renameFormRef = ref<FormInstance>()
const passwordFormRef = ref<FormInstance>()

const renameDialog = reactive({
  visible: false,
  loading: false,
  form: { id: 0, username: '' },
})
const passwordDialog = reactive({
  visible: false,
  loading: false,
  form: { id: 0, password: '' },
})

const deleteDialog = reactive({
  visible: false,
  loading: false,
  username: '',
  details: [] as { label: string; value: string | number }[],
})
type PlayerDetailAction = 'status' | 'webAccess' | 'kick'

const actionDialog = reactive({
  visible: false,
  loading: false,
  action: '' as PlayerDetailAction | '',
  title: '',
  message: '',
  confirmText: '',
  confirmType: 'warning' as 'primary' | 'success' | 'warning' | 'danger',
  details: [] as { label: string; value: string | number }[],
})
const renameRules: FormRules = {
  username: [{ required: true, message: () => t('player.validation.usernameRequired'), trigger: 'blur' },
             { min: 1, max: 30, message: () => t('player.validation.username'), trigger: 'blur' }],
}
const passwordRules: FormRules = {
  password: [{ required: true, message: () => t('player.validation.passwordRequired'), trigger: 'blur' },
             { min: 1, max: 15, message: () => t('player.validation.password'), trigger: 'blur' }],
}

const playerId = computed(() => Number(route.params.id))
const trafficRangeOptions = computed(() => [
  { label: t('player.trafficRange24h'), value: 24 },
  { label: t('player.trafficRange72h'), value: 72 },
  { label: t('player.trafficRange7d'), value: 168 },
  { label: t('player.trafficRange30d'), value: 720 },
])
const trafficChartItems = computed(() => {
  const byHour = new Map((trafficStats.value?.items ?? []).map(item => [item.hour, item]))
  const now = new Date()
  now.setUTCMinutes(0, 0, 0)

  return Array.from({ length: trafficHours.value }, (_, index) => {
    const hour = new Date(now.getTime() - (trafficHours.value - index - 1) * 3600 * 1000)
    const key = formatHourKey(hour)
    const item = byHour.get(key)
    return {
      hour: key,
      label: formatHourLabel(key),
      bytes_in: item?.bytes_in ?? 0,
      bytes_out: item?.bytes_out ?? 0,
    }
  })
})
const trafficChartOption = computed(() => {
  const items = trafficChartItems.value
  const isDark = appStore.theme === 'dark'
  const axisColor = isDark ? '#a5b4fc' : '#64748b'
  const gridColor = isDark ? 'rgba(129,140,248,.11)' : 'rgba(148,163,184,.22)'
  const axisLineColor = isDark ? 'rgba(129,140,248,.22)' : 'rgba(148,163,184,.34)'
  const tooltipBg = isDark ? 'rgba(10,8,35,.94)' : 'rgba(255,255,255,.96)'
  const tooltipText = isDark ? '#f8fafc' : '#0f172a'
  return {
    backgroundColor: 'transparent',
    color: ['#f97316', '#38bdf8'],
    tooltip: {
      trigger: 'axis',
      backgroundColor: tooltipBg,
      borderColor: 'rgba(249,115,22,.28)',
      textStyle: { color: tooltipText },
      valueFormatter: (value: number) => formatBytes(value),
    },
    legend: {
      top: 4,
      right: 0,
      textStyle: { color: axisColor },
      data: [t('player.trafficIn'), t('player.trafficOut')],
    },
    grid: {
      left: 10,
      right: 14,
      top: 42,
      bottom: 10,
      containLabel: true,
    },
    xAxis: {
      type: 'category',
      boundaryGap: false,
      data: items.map(item => item.label),
      axisLabel: {
        color: axisColor,
        hideOverlap: true,
      },
      axisLine: { lineStyle: { color: axisLineColor } },
      axisTick: { lineStyle: { color: axisLineColor } },
    },
    yAxis: {
      type: 'value',
      axisLabel: {
        color: axisColor,
        formatter: (value: number) => formatBytes(value),
      },
      splitLine: { lineStyle: { color: gridColor } },
    },
    series: [
      {
        name: t('player.trafficIn'),
        type: 'line',
        smooth: true,
        showSymbol: false,
        lineStyle: { width: 3 },
        emphasis: { focus: 'series' },
        areaStyle: {
          color: {
            type: 'linear',
            x: 0,
            y: 0,
            x2: 0,
            y2: 1,
            colorStops: [
              { offset: 0, color: 'rgba(249,115,22,.34)' },
              { offset: 1, color: 'rgba(249,115,22,.03)' },
            ],
          },
        },
        data: items.map(item => item.bytes_in),
      },
      {
        name: t('player.trafficOut'),
        type: 'line',
        smooth: true,
        showSymbol: false,
        lineStyle: { width: 2 },
        emphasis: { focus: 'series' },
        areaStyle: {
          color: {
            type: 'linear',
            x: 0,
            y: 0,
            x2: 0,
            y2: 1,
            colorStops: [
              { offset: 0, color: 'rgba(56,189,248,.20)' },
              { offset: 1, color: 'rgba(56,189,248,.02)' },
            ],
          },
        },
        data: items.map(item => item.bytes_out),
      },
    ],
  }
})

async function loadDetail() {
  if (!Number.isFinite(playerId.value) || playerId.value <= 0) {
    player.value = null
    return
  }

  loading.value = true
  trafficStats.value = null
  try {
    const res = await playerApi.detail({ id: playerId.value })
    player.value = res.data.player
    if (!player.value) {
      ElMessage.error(t('player.notFound'))
      return
    }
    loadTrafficStats()
  } finally {
    loading.value = false
  }
}

async function loadTrafficStats() {
  if (!Number.isFinite(playerId.value) || playerId.value <= 0) return
  trafficLoading.value = true
  try {
    const res = await playerApi.trafficStats({ user_id: playerId.value, hours: trafficHours.value })
    trafficStats.value = res.data
  } catch {
    trafficStats.value = null
  } finally {
    trafficLoading.value = false
  }
}

function openRenameDialog() {
  if (!player.value || !authStore.isAdmin) return
  renameDialog.form = { id: player.value.id, username: player.value.username }
  renameDialog.visible = true
}

function openPasswordDialog() {
  if (!player.value) return
  passwordDialog.form = { id: player.value.id, password: '' }
  passwordDialog.visible = true
}

async function handleRename() {
  const valid = await renameFormRef.value?.validate().catch(() => false)
  if (!valid) return
  renameDialog.loading = true
  try {
    const res = await playerApi.rename(renameDialog.form)
    if (res.data.code === 0) {
      ElMessage.success(t('player.saveSuccess'))
      renameDialog.visible = false
      loadDetail()
    } else {
      ElMessage.error(res.data.msg || t('common.failed'))
    }
  } finally {
    renameDialog.loading = false
  }
}

async function handleResetPassword() {
  const valid = await passwordFormRef.value?.validate().catch(() => false)
  if (!valid || !player.value) return
  if (!authStore.isAdmin) {
    await ElMessageBox.confirm(
      t('player.selfResetPasswordConfirm'),
      t('player.resetPasswordTitle'),
      { type: 'warning', confirmButtonText: t('common.confirm'), cancelButtonText: t('common.cancel') },
    )
  }
  passwordDialog.loading = true
  try {
    const res = await playerApi.resetPassword(passwordDialog.form)
    if (res.data.code === 0) {
      ElMessage.success(t('player.passwordResetSuccess'))
      passwordDialog.visible = false
      if (authStore.isAdmin) {
        loadDetail()
      } else {
        authStore.clearSession()
        router.replace('/login')
      }
    } else {
      ElMessage.error(res.data.msg || t('common.failed'))
    }
  } finally {
    passwordDialog.loading = false
  }
}

function handleToggleStatus() {
  if (!player.value || !authStore.isAdmin) return
  actionDialog.action = 'status'
  actionDialog.title = t(player.value.enabled ? 'player.disableTitle' : 'player.enableTitle')
  actionDialog.message = t(player.value.enabled ? 'player.disableConfirm' : 'player.enableConfirm', { name: player.value.username })
  actionDialog.confirmText = t(player.value.enabled ? 'common.disable' : 'common.enable')
  actionDialog.confirmType = player.value.enabled ? 'warning' : 'success'
  actionDialog.details = playerDetails()
  actionDialog.loading = false
  actionDialog.visible = true
}

function handleToggleWebAccess() {
  if (!player.value || !authStore.isAdmin) return
  actionDialog.action = 'webAccess'
  actionDialog.title = t(player.value.web_access ? 'player.revokeWebAccessTitle' : 'player.grantWebAccessTitle')
  actionDialog.message = t(player.value.web_access ? 'player.revokeWebAccessConfirm' : 'player.grantWebAccessConfirm', { name: player.value.username })
  actionDialog.confirmText = t('common.confirm')
  actionDialog.confirmType = player.value.web_access ? 'warning' : 'success'
  actionDialog.details = playerDetails()
  actionDialog.loading = false
  actionDialog.visible = true
}

function handleKick() {
  if (!player.value || !authStore.isAdmin) return
  actionDialog.action = 'kick'
  actionDialog.title = t('player.kickTitle')
  actionDialog.message = t('player.kickConfirm', { name: player.value.username })
  actionDialog.confirmText = t('common.confirm')
  actionDialog.confirmType = 'warning'
  actionDialog.details = playerDetails()
  actionDialog.loading = false
  actionDialog.visible = true
}

async function handleActionConfirm() {
  const target = player.value
  if (!target || !actionDialog.action) return

  actionDialog.loading = true
  try {
    if (actionDialog.action === 'status') {
      const wasEnabled = target.enabled
      const res = await playerApi.updateStatus({ id: target.id, enabled: wasEnabled ? 0 : 1 })
      if (res.data.code === 0) {
        ElMessage.success(wasEnabled ? t('player.disableSuccess') : t('player.enableSuccess'))
        actionDialog.visible = false
        loadDetail()
      } else {
        ElMessage.error(res.data.msg || t('common.failed'))
      }
      return
    }

    if (actionDialog.action === 'webAccess') {
      const hadWebAccess = target.web_access
      const res = await playerApi.updateWebAccess({ id: target.id, web_access: hadWebAccess ? 0 : 1 })
      if (res.data.code === 0) {
        ElMessage.success(hadWebAccess ? t('player.revokeWebAccessSuccess') : t('player.grantWebAccessSuccess'))
        actionDialog.visible = false
        loadDetail()
      } else {
        ElMessage.error(res.data.msg || t('common.failed'))
      }
      return
    }

    const res = await playerApi.kick({ id: target.id })
    if (res.data.code === 0) {
      ElMessage.success(t('player.kickSuccess'))
      actionDialog.visible = false
      loadDetail()
    } else {
      ElMessage.error(res.data.msg || t('common.failed'))
    }
  } finally {
    actionDialog.loading = false
  }
}

function handleRemove() {
  if (!player.value || !authStore.isAdmin) return
  const target = player.value
  deleteDialog.username = target.username
  deleteDialog.details = [
    { label: t('common.id'), value: String(target.id) },
    { label: t('player.username'), value: target.username },
    { label: t('player.table.status'), value: target.online ? t('common.online') : t('common.offline') },
  ]
  deleteDialog.loading = false
  deleteDialog.visible = true
}

function playerDetails() {
  if (!player.value) return []
  return [
    { label: t('common.id'), value: player.value.id },
    { label: t('player.username'), value: player.value.username },
    { label: t('player.table.status'), value: player.value.online ? t('common.online') : t('common.offline') },
  ]
}

async function handleRemoveConfirm() {
  if (!player.value) return
  deleteDialog.loading = true
  try {
    const res = await playerApi.remove({ id: player.value.id })
    if (res.data.code === 0) {
      ElMessage.success(t('player.deleteSuccess'))
      deleteDialog.visible = false
      router.push('/players')
    } else {
      ElMessage.error(res.data.msg || t('common.failed'))
    }
  } finally {
    deleteDialog.loading = false
  }
}

function formatBytes(n: number): string {
  if (n < 1024) return n + ' B'
  if (n < 1048576) return (n / 1024).toFixed(1) + ' KB'
  if (n < 1073741824) return (n / 1048576).toFixed(1) + ' MB'
  return (n / 1073741824).toFixed(2) + ' GB'
}

function formatHourKey(date: Date): string {
  const pad = (n: number) => n.toString().padStart(2, '0')
  return `${date.getUTCFullYear()}-${pad(date.getUTCMonth() + 1)}-${pad(date.getUTCDate())} ${pad(date.getUTCHours())}`
}

function formatHourLabel(hour: string): string {
  // hour 格式 "2026-06-02 06" (UTC)，转为浏览器本地时间显示
  const d = new Date(hour + ':00:00Z')
  const pad = (n: number) => n.toString().padStart(2, '0')
  if (trafficHours.value <= 24) return `${pad(d.getHours())}:00`
  return `${pad(d.getMonth() + 1)}/${pad(d.getDate())} ${pad(d.getHours())}:00`
}

function formatDuration(seconds: number): string {
  if (!seconds) return '0s'
  const h = Math.floor(seconds / 3600)
  const m = Math.floor((seconds % 3600) / 60)
  const s = seconds % 60
  if (h > 0) return `${h}h ${m}m`
  if (m > 0) return `${m}m ${s}s`
  return `${s}s`
}

watch(() => route.params.id, loadDetail)
onMounted(loadDetail)
</script>

<style scoped lang="scss">
.detail-head {
  align-items: flex-start;
}

.back-btn {
  margin: 0 0 4px -8px;
  padding-left: 8px;
}

.detail-grid {
  display: grid;
  gap: 16px;
}

.summary-panel {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 12px;
}

.summary-card {
  border: 1px solid var(--border-color);
  border-radius: 6px;
  padding: 12px 14px;
  background: var(--bg-primary);
  display: flex;
  flex-direction: column;
  gap: 8px;

  span {
    color: var(--text-secondary);
    font-size: 12px;
  }

  strong {
    color: var(--text-primary);
    font-size: 20px;
  }
}

.action-panel {
  .panel-head {
    margin-bottom: 12px;
  }
}

.action-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.traffic-head {
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}

.traffic-chart-wrap {
  border: 1px solid rgba(129,140,248,.18);
  border-radius: 8px;
  padding: 14px;
  background:
    radial-gradient(circle at 82% 14%, rgba(249,115,22,.15), transparent 28%),
    linear-gradient(180deg, #0d0932 0%, #12082d 48%, #080720 100%);
  box-shadow: inset 0 1px 0 rgba(255,255,255,.04);
}

.traffic-chart-wrap.is-light {
  border-color: rgba(249,115,22,.18);
  background:
    radial-gradient(circle at 82% 14%, rgba(249,115,22,.12), transparent 30%),
    linear-gradient(180deg, #fff8f1 0%, #ffffff 48%, #f8fafc 100%);
  box-shadow: inset 0 1px 0 rgba(255,255,255,.72), 0 1px 2px rgba(15,23,42,.04);
}

.traffic-total {
  display: flex;
  gap: 18px;
  flex-wrap: wrap;
  color: #a5b4fc;
  font-size: 12px;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', monospace;
}

:global(html[data-theme='light']) .traffic-total {
  color: #64748b;
}

.traffic-chart {
  width: 100%;
  height: 360px;
}

@media (max-width: 720px) {
  .summary-panel {
    grid-template-columns: 1fr;
  }

  .traffic-chart {
    height: 300px;
  }
}
</style>
