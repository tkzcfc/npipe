<template>
  <div class="page-container">
    <div class="page-title">
      <h2>{{ $t('dashboard.title') }}</h2>
      <span class="page-subtitle">{{ $t('dashboard.subtitle') }}</span>
    </div>

    <!-- Stat Cards -->
    <el-row :gutter="16" class="stat-row">
      <el-col :xs="12" :sm="12" :md="6" v-for="(card, idx) in statCards" :key="card.key">
        <div class="stat-card fade-in-up" :style="{ animationDelay: idx * 0.05 + 's' }">
          <div class="stat-icon" :style="{ background: card.iconBg }">
            <el-icon :size="22" :color="card.iconColor">
              <component :is="card.icon" />
            </el-icon>
          </div>
          <div class="stat-value">{{ card.value }}</div>
          <div class="stat-label">{{ card.label }}</div>
        </div>
      </el-col>
    </el-row>

    <!-- Charts Row -->
    <el-row :gutter="16" style="margin-top: 16px;">
      <el-col :md="12" :sm="24">
        <el-card>
          <template #header><span class="card-title">{{ $t('dashboard.userStatus') }}</span></template>
          <div ref="playerChartRef" class="chart-box" />
        </el-card>
      </el-col>
      <el-col :md="12" :sm="24">
        <el-card>
          <template #header><span class="card-title">{{ $t('dashboard.tunnelStatus') }}</span></template>
          <div ref="tunnelChartRef" class="chart-box" />
        </el-card>
      </el-col>
    </el-row>

    <!-- Recent Players -->
    <el-row :gutter="16" style="margin-top: 16px;">
      <el-col :span="24">
        <el-card>
          <template #header>
            <div class="flex-between">
              <span class="card-title">{{ $t('dashboard.recentUsers') }}</span>
              <el-button size="small" text @click="$router.push('/players')">{{ $t('common.viewAll') }}</el-button>
            </div>
          </template>
          <el-table :data="recentPlayers" size="small" v-loading="loadingStats">
            <el-table-column prop="id"       :label="$t('dashboard.table.id')"   width="70" />
            <el-table-column prop="username" :label="$t('dashboard.table.username')" />
            <el-table-column :label="$t('dashboard.table.status')" width="100">
              <template #default="{ row }">
                <el-tag :type="row.online ? 'success' : 'info'" size="small">
                  {{ row.online ? $t('common.online') : $t('common.offline') }}
                </el-tag>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import * as echarts from 'echarts'
import { playerApi, tunnelApi } from '@/api'
import { useAppStore } from '@/stores/app'
import type { Player, Tunnel } from '@/types'

const { t } = useI18n()
const appStore = useAppStore()

const loadingStats  = ref(false)
const recentPlayers = ref<Player[]>([])
const allTunnels    = ref<Tunnel[]>([])
const stats = ref({ onlinePlayers: 0, totalPlayers: 0, enabledTunnels: 0, totalTunnels: 0 })

const statCards = computed(() => [
  { key: 'onlinePlayers',  label: t('dashboard.onlineUsers'), value: stats.value.onlinePlayers,  icon: 'Connection', iconBg: 'rgba(63,185,80,.12)',  iconColor: '#3fb950' },
  { key: 'totalPlayers',   label: t('dashboard.totalUsers'),  value: stats.value.totalPlayers,   icon: 'User',       iconBg: 'rgba(88,166,255,.12)', iconColor: '#58a6ff' },
  { key: 'enabledTunnels', label: t('dashboard.enabledTunnels'), value: stats.value.enabledTunnels, icon: 'Share',      iconBg: 'rgba(210,153,34,.12)', iconColor: '#d29922' },
  { key: 'totalTunnels',   label: t('dashboard.totalTunnels'),  value: stats.value.totalTunnels,   icon: 'Link',       iconBg: 'rgba(248,81,73,.12)',  iconColor: '#f85149' },
])

// ── Charts ────────────────────────────────────────────────────────────────────
const playerChartRef = ref<HTMLDivElement | null>(null)
const tunnelChartRef = ref<HTMLDivElement | null>(null)
let playerChart: echarts.ECharts | null = null
let tunnelChart: echarts.ECharts | null = null

const isDark = computed(() => appStore.theme === 'dark')

function buildPieOption(data: { value: number; name: string; color: string }[]) {
  const dark = isDark.value
  return {
    backgroundColor: 'transparent',
    tooltip: {
      trigger: 'item',
      backgroundColor: dark ? '#1c2128' : '#fff',
      borderColor:     dark ? '#30363d' : '#e4e7ed',
      textStyle: { color: dark ? '#e6edf3' : '#303133' },
    },
    legend: { bottom: 8, textStyle: { color: dark ? '#8b949e' : '#606266' } },
    series: [{
      type: 'pie',
      radius: ['42%', '72%'],
      center: ['50%', '45%'],
      itemStyle: { borderRadius: 6, borderColor: dark ? '#161b22' : '#fff', borderWidth: 2 },
      label: { show: false },
      data: data.map(d => ({ value: d.value, name: d.name, itemStyle: { color: d.color } })),
    }],
  }
}

function renderPlayerChart(players: Player[]) {
  if (!playerChartRef.value) return
  if (!playerChart || playerChart.isDisposed()) playerChart = echarts.init(playerChartRef.value)
  playerChart.setOption(buildPieOption([
    { value: players.filter(p => p.online).length,  name: t('common.online'), color: '#3fb950' },
    { value: players.filter(p => !p.online).length, name: t('common.offline'), color: isDark.value ? '#30363d' : '#ddd' },
  ]))
}

function renderTunnelChart(tunnels: Tunnel[]) {
  if (!tunnelChartRef.value) return
  if (!tunnelChart || tunnelChart.isDisposed()) tunnelChart = echarts.init(tunnelChartRef.value)
  tunnelChart.setOption(buildPieOption([
    { value: tunnels.filter(t => t.enabled).length,  name: t('common.enable'), color: '#58a6ff' },
    { value: tunnels.filter(t => !t.enabled).length, name: t('common.disable'), color: '#f85149' },
  ]))
}

// ── Data ──────────────────────────────────────────────────────────────────────
async function loadData() {
  loadingStats.value = true
  try {
    const [pRes, tRes] = await Promise.all([
      playerApi.list({ page_number: 0, page_size: 10 }),
      tunnelApi.list({ page_number: 0, page_size: 200 }),
    ])
    const players: Player[] = pRes.data.players ?? []
    const tunnels: Tunnel[] = tRes.data.tunnels  ?? []

    recentPlayers.value         = players
    allTunnels.value             = tunnels
    stats.value.totalPlayers     = pRes.data.total_count ?? 0
    stats.value.totalTunnels     = tRes.data.total_count ?? 0
    stats.value.onlinePlayers    = players.filter(p => p.online).length
    stats.value.enabledTunnels   = tunnels.filter(t => t.enabled).length

    await nextTick()
    renderPlayerChart(players)
    renderTunnelChart(tunnels)
  } finally {
    loadingStats.value = false
  }
}

watch(() => appStore.theme, () => {
  renderPlayerChart(recentPlayers.value)
  renderTunnelChart(allTunnels.value)
})

function onResize() { playerChart?.resize(); tunnelChart?.resize() }

onMounted(() => { loadData(); window.addEventListener('resize', onResize) })
onUnmounted(() => {
  window.removeEventListener('resize', onResize)
  playerChart?.dispose()
  tunnelChart?.dispose()
})
</script>

<style scoped lang="scss">
.page-title {
  margin-bottom: 20px;
  h2 { margin: 0 0 2px; font-size: 20px; font-weight: 700; }
  .page-subtitle { font-size: 13px; color: var(--text-muted); }
}
.card-title { font-size: 14px; font-weight: 600; }
.chart-box  { width: 100%; height: 230px; }
</style>
