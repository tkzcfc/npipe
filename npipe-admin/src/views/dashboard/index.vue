<template>
  <div class="page-container">
    <div class="page-head">
      <div>
        <h1>{{ $t('dashboard.title') }}</h1>
        <p>{{ $t('dashboard.subtitle') }}</p>
      </div>
      <el-button :icon="Refresh" @click="loadData(false)" :loading="loading">{{ $t('common.refresh') }}</el-button>
    </div>

    <div class="metric-grid">
      <button v-for="card in statCards" :key="card.key" class="metric-card" @click="router.push(card.path)">
        <div class="metric-top">
          <span class="metric-label">{{ card.label }}</span>
          <el-icon :color="card.color"><component :is="card.icon" /></el-icon>
        </div>
        <div class="metric-value">{{ card.value }}</div>
        <div class="metric-note">{{ card.note }}</div>
      </button>
    </div>

    <section class="panel">
      <div class="panel-head">
        <h2>{{ $t('dashboard.resourceUsage') }}</h2>
      </div>
      <div class="resource-grid" v-loading="loading">
        <div class="resource-card">
          <div class="resource-head">
            <span>{{ $t('dashboard.system.cpuUsage') }}</span>
            <strong>{{ formatPercent(system.cpu_usage) }}</strong>
          </div>
          <el-progress :percentage="normalizePercent(system.cpu_usage)" :stroke-width="10" />
          <div class="resource-meta">{{ system.cpu_cores }} {{ $t('dashboard.system.cpuCores') }}</div>
        </div>
        <div class="resource-card">
          <div class="resource-head">
            <span>{{ $t('dashboard.system.memoryUsage') }}</span>
            <strong>{{ formatPercent(system.memory_usage) }}</strong>
          </div>
          <el-progress :percentage="normalizePercent(system.memory_usage)" :stroke-width="10" />
          <div class="resource-meta">
            {{ formatBytes(system.used_memory) }} / {{ formatBytes(system.total_memory) }}
          </div>
        </div>
      </div>
    </section>

    <div class="info-grid">
      <section class="panel">
        <div class="panel-head">
          <h2>{{ $t('dashboard.machineInfo') }}</h2>
        </div>
        <div class="config-grid compact" v-loading="loading">
          <div v-for="item in systemItems" :key="item.label" class="config-item">
            <span class="config-label">{{ item.label }}</span>
            <span class="config-value" :class="{ muted: !item.value }">{{ item.value || '-' }}</span>
          </div>
        </div>
      </section>

      <section class="panel">
        <div class="panel-head">
          <h2>{{ $t('dashboard.serverConfig') }}</h2>
        </div>
        <div class="config-grid compact" v-loading="loading">
          <div v-for="item in configItems" :key="item.label" class="config-item">
            <span class="config-label">{{ item.label }}</span>
            <span class="config-value" :class="{ muted: !item.value }">{{ item.value || '-' }}</span>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { Refresh } from '@element-plus/icons-vue'
import { dashboardApi } from '@/api'
import type { DashboardConfigInfo, DashboardSystemInfo } from '@/types'

const { t } = useI18n()
const router = useRouter()

const loading = ref(false)
const stats = ref({ onlinePlayers: 0, totalPlayers: 0, enabledTunnels: 0, totalTunnels: 0 })
const config = ref<DashboardConfigInfo>({
  listen_addr: '',
  web_addr: '',
  enable_tls: false,
  tls_cert: '',
  web_base_dir: '',
  illegal_traffic_forward: '',
  quiet: false,
  log_dir: '',
  database: '',
})
const system = ref<DashboardSystemInfo>({
  host_name: '',
  os_name: '',
  kernel_version: '',
  uptime_secs: 0,
  cpu_usage: 0,
  cpu_cores: 0,
  total_memory: 0,
  used_memory: 0,
  memory_usage: 0,
})

const statCards = computed(() => [
  { key: 'onlinePlayers', label: t('dashboard.onlineUsers'), value: stats.value.onlinePlayers, note: `${t('dashboard.totalUsers')}: ${stats.value.totalPlayers}`, icon: 'UserFilled', color: '#16a34a', path: '/players' },
  { key: 'totalPlayers', label: t('dashboard.totalUsers'), value: stats.value.totalPlayers, note: t('player.subtitle'), icon: 'User', color: '#2563eb', path: '/players' },
  { key: 'enabledTunnels', label: t('dashboard.enabledTunnels'), value: stats.value.enabledTunnels, note: `${t('dashboard.totalTunnels')}: ${stats.value.totalTunnels}`, icon: 'Connection', color: '#d97706', path: '/tunnels' },
  { key: 'totalTunnels', label: t('dashboard.totalTunnels'), value: stats.value.totalTunnels, note: t('tunnel.subtitle'), icon: 'Share', color: '#7c3aed', path: '/tunnels' },
])

const configItems = computed(() => [
  { label: t('dashboard.config.listenAddr'), value: config.value.listen_addr },
  { label: t('dashboard.config.webAddr'), value: config.value.web_addr },
  { label: t('dashboard.config.tls'), value: config.value.enable_tls ? t('common.enable') : t('common.disable') },
  { label: t('dashboard.config.tlsCert'), value: config.value.tls_cert },
  { label: t('dashboard.config.database'), value: config.value.database },
  { label: t('dashboard.config.webBaseDir'), value: config.value.web_base_dir },
  { label: t('dashboard.config.forward'), value: config.value.illegal_traffic_forward },
  { label: t('dashboard.config.logDir'), value: config.value.log_dir },
  { label: t('dashboard.config.quiet'), value: config.value.quiet ? t('common.enable') : t('common.disable') },
])

const systemItems = computed(() => [
  { label: t('dashboard.system.hostName'), value: system.value.host_name },
  { label: t('dashboard.system.osName'), value: system.value.os_name },
  { label: t('dashboard.system.kernelVersion'), value: system.value.kernel_version },
  { label: t('dashboard.system.uptime'), value: formatDuration(system.value.uptime_secs) },
  { label: t('dashboard.system.cpuCores'), value: String(system.value.cpu_cores || '') },
  { label: t('dashboard.system.totalMemory'), value: formatBytes(system.value.total_memory) },
])

let refreshing = false

async function loadData(silent = false) {
  if (refreshing) return
  refreshing = true
  if (!silent) loading.value = true
  try {
    const res = await dashboardApi.overview()
    stats.value = {
      onlinePlayers: res.data.online_players,
      totalPlayers: res.data.total_players,
      enabledTunnels: res.data.enabled_tunnels,
      totalTunnels: res.data.total_tunnels,
    }
    config.value = res.data.config
    system.value = res.data.system
  } finally {
    if (!silent) loading.value = false
    refreshing = false
  }
}

function normalizePercent(value: number) {
  return Math.max(0, Math.min(100, Number(value.toFixed(1))))
}

function formatPercent(value: number) {
  return `${normalizePercent(value).toFixed(1)}%`
}

function formatBytes(bytes: number) {
  if (!bytes) return ''
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let value = bytes
  let unit = 0
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024
    unit += 1
  }
  return `${value.toFixed(unit <= 1 ? 0 : 1)} ${units[unit]}`
}

function formatDuration(totalSeconds: number) {
  if (!totalSeconds) return ''
  const days = Math.floor(totalSeconds / 86400)
  const hours = Math.floor((totalSeconds % 86400) / 3600)
  const minutes = Math.floor((totalSeconds % 3600) / 60)
  if (days > 0) return `${days}d ${hours}h`
  if (hours > 0) return `${hours}h ${minutes}m`
  return `${minutes}m`
}

let refreshTimer: number | undefined

onMounted(() => {
  loadData(false)
  refreshTimer = window.setInterval(() => loadData(true), 10000)
})

onUnmounted(() => {
  if (refreshTimer) window.clearInterval(refreshTimer)
})
</script>

<style scoped lang="scss">
.metric-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 14px;
  margin-bottom: 16px;
}

.metric-card {
  text-align: left;
  background: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  padding: 18px;
  box-shadow: var(--shadow-sm);
  cursor: pointer;
  transition: border-color .18s, box-shadow .18s, transform .18s;

  &:hover {
    border-color: var(--accent);
    box-shadow: var(--shadow-md);
    transform: translateY(-1px);
  }
}

.metric-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  color: var(--text-secondary);
}

.metric-label { font-size: 13px; }
.metric-value { margin-top: 10px; font-size: 30px; font-weight: 760; line-height: 1; }
.metric-note { margin-top: 8px; font-size: 12px; color: var(--text-muted); }

.resource-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}

.resource-card {
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: var(--bg-primary);
  padding: 16px;
}

.resource-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 12px;
  color: var(--text-secondary);

  strong {
    color: var(--text-primary);
    font-size: 18px;
  }
}

.resource-meta {
  margin-top: 10px;
  color: var(--text-muted);
  font-size: 12px;
  font-family: 'JetBrains Mono','Fira Code','Cascadia Code',Consolas,monospace;
}

.info-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 16px;
  margin-top: 16px;
}

.config-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 12px;
}

.config-grid.compact {
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.config-item {
  min-height: 64px;
  padding: 12px;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: var(--bg-primary);
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: 5px;
}

.config-label {
  font-size: 12px;
  color: var(--text-muted);
}

.config-value {
  font-family: 'JetBrains Mono','Fira Code','Cascadia Code',Consolas,monospace;
  font-size: 13px;
  color: var(--text-primary);
  word-break: break-all;
}

.config-value.muted {
  color: var(--text-muted);
}

@media (max-width: 1100px) {
  .metric-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .resource-grid,
  .info-grid { grid-template-columns: 1fr; }
  .config-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
}

@media (max-width: 640px) {
  .metric-grid,
  .resource-grid,
  .info-grid,
  .config-grid { grid-template-columns: 1fr; }
}
</style>
