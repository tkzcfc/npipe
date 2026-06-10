<template>
  <div class="page-container">
    <div class="page-head">
      <div>
        <h1>{{ $t('dashboard.title') }}</h1>
        <p>{{ $t('dashboard.subtitle') }}</p>
      </div>
      <el-button :icon="Refresh" @click="loadData(false)" :loading="loading">{{ $t('common.refresh') }}</el-button>
    </div>

    <!-- 统计卡片 -->
    <section class="panel stat-panel">
      <div class="stat-grid">
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

    <!-- 机器信息 + 服务配置 -->
    <div class="main-grid">
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
          <div class="resource-row">
            <div class="resource-card">
              <div class="resource-head">
                <span>{{ $t('dashboard.system.cpuUsage') }}</span>
                <strong>{{ formatPercent(system.cpu_usage) }}</strong>
              </div>
              <el-progress :percentage="normalizePercent(system.cpu_usage)" :stroke-width="8" />
              <div class="resource-meta">{{ system.cpu_cores }} {{ $t('dashboard.system.cpuCores') }}</div>
            </div>
            <div class="resource-card">
              <div class="resource-head">
                <span>{{ $t('dashboard.system.memoryUsage') }}</span>
                <strong>{{ formatPercent(system.memory_usage) }}</strong>
              </div>
              <el-progress :percentage="normalizePercent(system.memory_usage)" :stroke-width="8" />
              <div class="resource-meta">{{ formatBytes(system.used_memory) }} / {{ formatBytes(system.total_memory) }}</div>
            </div>
          </div>
          <div class="machine-facts">
            <div class="fact-item">
              <span>{{ $t('dashboard.system.kernelVersion') }}</span>
              <strong>{{ system.kernel_version || '-' }}</strong>
            </div>
          </div>
        </div>
      </section>

      <section class="panel service-panel">
        <div class="panel-head">
          <h2>{{ $t('dashboard.serverConfig') }}</h2>
        </div>
        <div class="service-layout" v-loading="loading">
          <div class="service-toggle-grid">
            <div v-for="item in serviceStatusItems" :key="item.label" class="service-toggle" :class="{ enabled: item.enabled }">
              <span>{{ item.label }}</span>
              <strong>{{ item.enabled ? $t('common.enable') : $t('common.disable') }}</strong>
            </div>
          </div>
          <div class="config-table">
            <div v-for="item in configTableItems" :key="item.label" class="config-row">
              <span class="config-label">{{ item.label }}</span>
              <code class="config-value" :class="{ muted: !item.value }">{{ item.value || '-' }}</code>
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
  transport_max_connections_per_player: 0,
  transport_idle_timeout_secs: 0,
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

const configTableItems = computed(() => [
  { label: t('dashboard.config.listenAddr'), value: config.value.listen_addr },
  { label: t('dashboard.config.webAddr'), value: config.value.web_addr },
  { label: t('dashboard.config.database'), value: config.value.database },
  { label: t('dashboard.config.webBaseDir'), value: config.value.web_base_dir },
  { label: t('dashboard.config.logDir'), value: config.value.log_dir },
  { label: t('dashboard.config.forward'), value: config.value.illegal_traffic_forward },
  { label: t('dashboard.config.tunnelTlsCert'), value: config.value.tls_cert },
  { label: t('dashboard.config.webTlsCert'), value: config.value.web_tls_cert },
  { label: t('dashboard.config.transportMaxConnections'), value: String(config.value.transport_max_connections_per_player) },
  { label: t('dashboard.config.transportIdleTimeout'), value: `${config.value.transport_idle_timeout_secs}s` },
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
/* ─── 统计卡片（横排4个） ─── */
.stat-panel {
  margin-bottom: 16px;
}

.stat-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
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

/* ─── 主体两栏布局 ─── */
.main-grid {
  display: grid;
  grid-template-columns: 380px minmax(0, 1fr);
  gap: 16px;
}

/* ─── 机器信息 ─── */
.machine-layout {
  display: grid;
  gap: 12px;
}

.machine-hero {
  border: 1px solid rgba(91, 143, 249, .28);
  border-radius: 8px;
  padding: 18px;
  min-height: 120px;
  background:
    linear-gradient(135deg, rgba(91, 143, 249, .16), rgba(22, 163, 74, .07) 52%, rgba(217, 119, 6, .08)),
    var(--bg-primary);
  display: flex;
  flex-direction: column;
  justify-content: flex-end;
  gap: 6px;

  strong {
    color: var(--text-primary);
    font-size: 22px;
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
  padding: 3px 8px;
  border-radius: 5px;
  background: rgba(255, 255, 255, .06);
  border: 1px solid rgba(255, 255, 255, .08);
  color: var(--text-primary) !important;
  font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', Consolas, monospace;
  font-size: 12px;
}

.resource-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
}

.resource-card {
  border: 1px solid var(--border-color);
  border-radius: 7px;
  background: var(--bg-primary);
  padding: 14px 14px 12px;
}

.resource-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin-bottom: 10px;
  color: var(--text-secondary);
  font-size: 12px;

  strong {
    color: var(--text-primary);
    font-size: 16px;
  }
}

.resource-meta {
  margin-top: 8px;
  color: var(--text-muted);
  font-size: 11px;
  font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', Consolas, monospace;
}

.machine-facts {
  display: grid;
  gap: 8px;
}

.fact-item {
  padding: 10px 12px;
  border: 1px solid var(--border-color);
  border-radius: 7px;
  background: var(--bg-primary);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;

  span {
    font-size: 12px;
    color: var(--text-muted);
  }

  strong {
    font-size: 12px;
    color: var(--text-primary);
    font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', Consolas, monospace;
  }
}

/* ─── 服务配置 ─── */
.service-layout {
  display: grid;
  gap: 14px;
}

.service-toggle-grid {
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
  gap: 8px;
}

.service-toggle {
  padding: 10px;
  border: 1px solid var(--border-color);
  border-radius: 7px;
  background:
    linear-gradient(180deg, rgba(148, 163, 184, .06), transparent),
    var(--bg-primary);
  display: flex;
  flex-direction: column;
  gap: 6px;

  span {
    color: var(--text-muted);
    font-size: 11px;
    line-height: 1.3;
  }

  strong {
    color: var(--text-secondary);
    font-size: 12px;
  }

  &.enabled {
    border-color: rgba(22, 163, 74, .32);
    background:
      linear-gradient(180deg, rgba(22, 163, 74, .10), transparent),
      var(--bg-primary);

    strong {
      color: #22c55e;
    }
  }
}

.config-table {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
}

.config-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 9px 12px;
  border: 1px solid var(--border-color);
  border-radius: 7px;
  background: var(--bg-primary);
  min-height: 38px;

  .config-label {
    flex-shrink: 0;
    font-size: 11px;
    color: var(--text-muted);
    white-space: nowrap;
  }

  .config-value {
    flex: 1;
    min-width: 0;
    font-size: 12px;
    color: var(--text-primary);
    font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', Consolas, monospace;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
}

.muted {
  color: var(--text-muted) !important;
}

/* ─── 响应式 ─── */
@media (max-width: 1200px) {
  .main-grid {
    grid-template-columns: 1fr;
  }
  .config-table {
    grid-template-columns: 1fr;
  }
}

@media (max-width: 900px) {
  .stat-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
  .service-toggle-grid {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }
}

@media (max-width: 640px) {
  .stat-grid,
  .resource-row,
  .service-toggle-grid,
  .config-table {
    grid-template-columns: 1fr;
  }
}
</style>
