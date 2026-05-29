<template>
  <div class="page-container">
    <div class="page-head">
      <div>
        <h1>{{ $t('dashboard.title') }}</h1>
        <p>{{ $t('dashboard.subtitle') }}</p>
      </div>
      <el-button :icon="Refresh" @click="loadData(false)" :loading="loading">{{ $t('common.refresh') }}</el-button>
    </div>

    <div class="overview-layout">
      <section class="panel status-panel">
        <div class="panel-head">
          <h2>{{ $t('dashboard.userAndTunnel') }}</h2>
        </div>
        <div class="metric-grid">
          <button v-for="card in statCards" :key="card.key" class="metric-card" @click="router.push(card.path)">
            <span class="metric-icon" :style="{ color: card.color, background: card.bg }">
              <el-icon><component :is="card.icon" /></el-icon>
            </span>
            <span class="metric-label">{{ card.label }}</span>
            <strong class="metric-value">{{ card.value }}</strong>
            <span class="metric-note">{{ card.note }}</span>
          </button>
        </div>
      </section>

      <section class="panel resource-panel">
        <div class="panel-head">
          <h2>{{ $t('dashboard.serverStatus') }}</h2>
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
    </div>

    <div class="info-grid">
      <section class="panel machine-panel">
        <div class="panel-head">
          <h2>{{ $t('dashboard.machineInfo') }}</h2>
        </div>
        <div class="machine-layout" v-loading="loading">
          <div class="machine-hero">
            <span class="machine-kicker">{{ system.os_name || '-' }}</span>
            <strong>{{ system.host_name || '-' }}</strong>
            <span>{{ $t('dashboard.system.uptime') }} {{ formatDuration(system.uptime_secs) || '-' }}</span>
          </div>
          <div class="machine-facts">
            <div v-for="item in systemItems" :key="item.label" class="fact-item">
              <span>{{ item.label }}</span>
              <strong :class="{ muted: !item.value }">{{ item.value || '-' }}</strong>
            </div>
          </div>
        </div>
      </section>

      <section class="panel service-panel">
        <div class="panel-head">
          <h2>{{ $t('dashboard.serverConfig') }}</h2>
        </div>
        <div class="service-layout" v-loading="loading">
          <div class="service-status-grid">
            <div v-for="item in serviceStatusItems" :key="item.label" class="service-status" :class="{ enabled: item.enabled }">
              <span>{{ item.label }}</span>
              <strong>{{ item.enabled ? $t('common.enable') : $t('common.disable') }}</strong>
            </div>
          </div>

          <div class="service-path-list">
            <div v-for="item in servicePathItems" :key="item.label" class="service-path">
              <span>{{ item.label }}</span>
              <code :class="{ muted: !item.value }">{{ item.value || '-' }}</code>
            </div>
          </div>

          <div class="service-extra-grid">
            <div v-for="item in serviceExtraItems" :key="item.label" class="mini-config">
              <span>{{ item.label }}</span>
              <strong :class="{ muted: !item.value }">{{ item.value || '-' }}</strong>
            </div>
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
import { Connection, Refresh, Share, User, UserFilled } from '@element-plus/icons-vue'
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
  web_enable_tls: false,
  web_tls_cert: '',
  web_tls_auto_self_signed: false,
  web_cookie_secure: false,
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
  { key: 'onlinePlayers', label: t('dashboard.onlineUsers'), value: stats.value.onlinePlayers, note: `${t('dashboard.totalUsers')}: ${stats.value.totalPlayers}`, icon: UserFilled, color: '#16a34a', bg: 'rgba(22,163,74,.12)', path: '/players' },
  { key: 'totalPlayers', label: t('dashboard.totalUsers'), value: stats.value.totalPlayers, note: t('player.subtitle'), icon: User, color: '#2563eb', bg: 'rgba(37,99,235,.12)', path: '/players' },
  { key: 'enabledTunnels', label: t('dashboard.enabledTunnels'), value: stats.value.enabledTunnels, note: `${t('dashboard.totalTunnels')}: ${stats.value.totalTunnels}`, icon: Connection, color: '#d97706', bg: 'rgba(217,119,6,.13)', path: '/tunnels' },
  { key: 'totalTunnels', label: t('dashboard.totalTunnels'), value: stats.value.totalTunnels, note: t('tunnel.subtitle'), icon: Share, color: '#7c3aed', bg: 'rgba(124,58,237,.12)', path: '/tunnels' },
])

const serviceStatusItems = computed(() => [
  { label: t('dashboard.config.tunnelTls'), enabled: config.value.enable_tls },
  { label: t('dashboard.config.webTls'), enabled: config.value.web_enable_tls },
  { label: t('dashboard.config.webTlsAutoSelfSigned'), enabled: config.value.web_tls_auto_self_signed },
  { label: t('dashboard.config.cookieSecure'), enabled: config.value.web_cookie_secure },
  { label: t('dashboard.config.quiet'), enabled: config.value.quiet },
])

const servicePathItems = computed(() => [
  { label: t('dashboard.config.listenAddr'), value: config.value.listen_addr },
  { label: t('dashboard.config.webAddr'), value: config.value.web_addr },
  { label: t('dashboard.config.database'), value: config.value.database },
  { label: t('dashboard.config.webBaseDir'), value: config.value.web_base_dir },
  { label: t('dashboard.config.logDir'), value: config.value.log_dir },
])

const serviceExtraItems = computed(() => [
  { label: t('dashboard.config.forward'), value: config.value.illegal_traffic_forward },
  { label: t('dashboard.config.tunnelTlsCert'), value: config.value.tls_cert },
  { label: t('dashboard.config.webTlsCert'), value: config.value.web_tls_cert },
])

const systemItems = computed(() => [
  { label: t('dashboard.system.kernelVersion'), value: system.value.kernel_version },
  { label: t('dashboard.system.cpuCores'), value: String(system.value.cpu_cores || '') },
  { label: t('dashboard.system.totalMemory'), value: formatBytes(system.value.total_memory) },
  { label: t('dashboard.system.memoryUsage'), value: formatPercent(system.value.memory_usage) },
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
.overview-layout {
  display: grid;
  grid-template-columns: minmax(0, 1.05fr) minmax(360px, .95fr);
  gap: 16px;
  margin-bottom: 16px;
}

.status-panel,
.resource-panel {
  min-height: 236px;
}

.metric-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}

.metric-card {
  min-height: 84px;
  text-align: left;
  background: var(--bg-primary);
  border: 1px solid var(--border-color);
  border-radius: 7px;
  padding: 13px 14px;
  cursor: pointer;
  display: grid;
  grid-template-columns: 36px 1fr;
  grid-template-rows: auto auto auto;
  column-gap: 12px;
  transition: border-color .18s, background .18s, transform .18s;

  &:hover {
    border-color: var(--accent);
    background: var(--bg-card);
    transform: translateY(-1px);
  }
}

.metric-icon {
  grid-row: 1 / 4;
  width: 36px;
  height: 36px;
  border-radius: 7px;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-top: 2px;
}

.metric-label {
  font-size: 12px;
  color: var(--text-secondary);
}

.metric-value {
  margin-top: 4px;
  font-size: 26px;
  font-weight: 760;
  line-height: 1;
  color: var(--text-primary);
}

.metric-note {
  margin-top: 6px;
  font-size: 12px;
  color: var(--text-muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.resource-grid {
  display: grid;
  grid-template-columns: 1fr;
  gap: 12px;
}

.resource-card {
  border: 1px solid var(--border-color);
  border-radius: 7px;
  background: var(--bg-primary);
  padding: 16px 16px 14px;
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
  grid-template-columns: minmax(360px, .82fr) minmax(0, 1.18fr);
  gap: 16px;
  margin-top: 16px;
}

.machine-layout,
.service-layout {
  display: grid;
  gap: 12px;
}

.machine-hero {
  min-height: 142px;
  border: 1px solid rgba(91,143,249,.28);
  border-radius: 8px;
  padding: 18px;
  background:
    linear-gradient(135deg, rgba(91,143,249,.16), rgba(22,163,74,.07) 52%, rgba(217,119,6,.08)),
    var(--bg-primary);
  display: flex;
  flex-direction: column;
  justify-content: flex-end;
  gap: 8px;

  strong {
    color: var(--text-primary);
    font-size: 24px;
    line-height: 1.15;
    word-break: break-word;
  }

  span {
    color: var(--text-secondary);
    font-size: 12px;
  }
}

.machine-kicker {
  width: fit-content;
  padding: 4px 8px;
  border-radius: 6px;
  background: rgba(255,255,255,.06);
  border: 1px solid rgba(255,255,255,.08);
  color: var(--text-primary) !important;
  font-family: 'JetBrains Mono','Fira Code','Cascadia Code',Consolas,monospace;
}

.machine-facts {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}

.fact-item,
.mini-config {
  min-height: 60px;
  padding: 11px 12px;
  border: 1px solid var(--border-color);
  border-radius: 7px;
  background: var(--bg-primary);
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: 5px;

  span {
    font-size: 12px;
    color: var(--text-muted);
  }

  strong {
    font-size: 13px;
    color: var(--text-primary);
    word-break: break-all;
    font-family: 'JetBrains Mono','Fira Code','Cascadia Code',Consolas,monospace;
  }
}

.service-status-grid {
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
  gap: 10px;
}

.service-status {
  min-height: 66px;
  padding: 11px;
  border: 1px solid var(--border-color);
  border-radius: 7px;
  background:
    linear-gradient(180deg, rgba(148,163,184,.08), transparent),
    var(--bg-primary);
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  gap: 8px;

  span {
    color: var(--text-muted);
    font-size: 12px;
  }

  strong {
    color: var(--text-secondary);
    font-size: 13px;
  }

  &.enabled {
    border-color: rgba(22,163,74,.32);
    background:
      linear-gradient(180deg, rgba(22,163,74,.13), transparent),
      var(--bg-primary);

    strong {
      color: #22c55e;
    }
  }
}

.service-path-list {
  display: grid;
  gap: 8px;
}

.service-path {
  min-height: 42px;
  display: grid;
  grid-template-columns: 130px minmax(0, 1fr);
  align-items: center;
  gap: 12px;
  padding: 9px 12px;
  border: 1px solid var(--border-color);
  border-radius: 7px;
  background: var(--bg-primary);

  span {
    color: var(--text-muted);
    font-size: 12px;
  }

  code {
    color: var(--text-primary);
    font-family: 'JetBrains Mono','Fira Code','Cascadia Code',Consolas,monospace;
    font-size: 12px;
    line-height: 1.55;
    word-break: break-all;
  }
}

.service-extra-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 10px;
}

.muted {
  color: var(--text-muted) !important;
}

@media (max-width: 1100px) {
  .overview-layout,
  .info-grid { grid-template-columns: 1fr; }
  .service-status-grid { grid-template-columns: repeat(3, minmax(0, 1fr)); }
}

@media (max-width: 640px) {
  .overview-layout,
  .metric-grid,
  .resource-grid,
  .info-grid,
  .machine-facts,
  .service-status-grid,
  .service-extra-grid,
  .service-path { grid-template-columns: 1fr; }
}
</style>
