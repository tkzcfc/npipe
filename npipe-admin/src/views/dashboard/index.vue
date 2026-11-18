<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { dashboardApi } from '@/api'
import type { DashboardConfigInfo, DashboardSystemInfo } from '@/types'
import { formatBytes, formatDuration, formatPercent } from '@/utils/format'
import PageHeader from '@/components/PageHeader.vue'
import Button from '@/components/ui/Button.vue'
import EmptyState from '@/components/ui/EmptyState.vue'
import Skeleton from '@/components/ui/Skeleton.vue'
import StatusBadge from '@/components/StatusBadge.vue'
import Icon from '@/components/ui/Icon.vue'

const { t } = useI18n()
const router = useRouter()

const loading = ref(false)
const loadError = ref('')
const loaded = ref(false)
const stats = ref({ onlinePlayers: 0, totalPlayers: 0, enabledTunnels: 0, totalTunnels: 0 })
const config = ref<DashboardConfigInfo | null>(null)
const system = ref<DashboardSystemInfo | null>(null)
let refreshing = false
let refreshTimer: number | undefined

const cards = computed(() => [
  { key: 'online', label: t('dashboard.onlineUsers'), value: stats.value.onlinePlayers, note: `${t('dashboard.totalUsers')}: ${stats.value.totalPlayers}`, icon: 'user', path: '/players' },
  { key: 'users', label: t('dashboard.totalUsers'), value: stats.value.totalPlayers, note: t('player.subtitle'), icon: 'users', path: '/players' },
  { key: 'enabled', label: t('dashboard.enabledTunnels'), value: stats.value.enabledTunnels, note: `${t('dashboard.totalTunnels')}: ${stats.value.totalTunnels}`, icon: 'tunnel', path: '/tunnels' },
  { key: 'tunnels', label: t('dashboard.totalTunnels'), value: stats.value.totalTunnels, note: t('tunnel.subtitle'), icon: 'link', path: '/tunnels' },
])

const flags = computed(() => {
  const c = config.value
  if (!c) return []
  return [
    { label: t('dashboard.config.tunnelTls'), enabled: c.enable_tls },
    { label: t('dashboard.config.webTls'), enabled: c.web_enable_tls },
    { label: t('dashboard.config.webTlsAutoSelfSigned'), enabled: c.web_tls_auto_self_signed },
    { label: t('dashboard.config.cookieSecure'), enabled: c.web_cookie_secure },
    { label: t('dashboard.config.quiet'), enabled: c.quiet },
  ]
})

const configRows = computed(() => {
  const c = config.value
  if (!c) return []
  return [
    [t('dashboard.config.listenAddr'), c.listen_addr],
    [t('dashboard.config.webAddr'), c.web_addr],
    [t('dashboard.config.database'), c.database],
    [t('dashboard.config.webBaseDir'), c.web_base_dir],
    [t('dashboard.config.logDir'), c.log_dir],
    [t('dashboard.config.forward'), c.illegal_traffic_forward || t('common.none')],
    [t('dashboard.config.tunnelTlsCert'), c.tls_cert],
    [t('dashboard.config.webTlsCert'), c.web_tls_cert],
    [t('dashboard.config.transportMaxConnections'), String(c.transport_max_connections_per_player)],
    [t('dashboard.config.transportIdleTimeout'), `${c.transport_idle_timeout_secs}s`],
  ]
})

async function loadData(silent = false) {
  if (refreshing) return
  refreshing = true
  if (!silent) {
    loading.value = true
    loadError.value = ''
  }
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
    loaded.value = true
    loadError.value = ''
  } catch {
    if (!silent || !loaded.value) loadError.value = t('common.loadFailed')
  } finally {
    if (!silent) loading.value = false
    refreshing = false
  }
}

onMounted(() => {
  loadData(false)
  refreshTimer = window.setInterval(() => loadData(true), 10000)
})
onUnmounted(() => {
  if (refreshTimer) window.clearInterval(refreshTimer)
})
</script>

<template>
  <div>
    <PageHeader :title="t('dashboard.title')" :subtitle="t('dashboard.subtitle')">
      <p class="text-xs text-faint">{{ t('dashboard.lastUpdated') }}</p>
      <Button icon="refresh" :loading="loading" @click="loadData(false)">{{ t('common.refresh') }}</Button>
    </PageHeader>

    <EmptyState v-if="loadError && !loaded" :title="t('common.loadFailed')" :description="loadError" icon="warning" retry @retry="loadData(false)" />

    <template v-else>
      <div class="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        <button
          v-for="card in cards"
          :key="card.key"
          type="button"
          class="ui-card p-4 text-left transition hover:border-accent/40"
          @click="router.push(card.path)"
        >
          <div class="flex items-start justify-between">
            <p class="text-sm text-muted">{{ card.label }}</p>
            <span class="flex h-8 w-8 items-center justify-center rounded-lg bg-accent-dim text-accent">
              <Icon :name="card.icon" :size="16" />
            </span>
          </div>
          <Skeleton v-if="loading && !loaded" class="mt-3 h-8 w-20" />
          <p v-else class="mt-2 text-3xl font-semibold tabular tracking-tight">{{ card.value }}</p>
          <p class="mt-1 truncate text-xs text-faint">{{ card.note }}</p>
        </button>
      </div>

      <div class="mt-5 grid gap-4 xl:grid-cols-5">
        <section class="ui-card p-5 xl:col-span-2">
          <h2 class="text-sm font-semibold">{{ t('dashboard.machineInfo') }}</h2>
          <div v-if="system" class="mt-4 space-y-4">
            <div>
              <div class="mb-1 flex justify-between text-xs text-muted">
                <span>{{ t('dashboard.system.cpuUsage') }}</span>
                <span class="tabular">{{ formatPercent(system.cpu_usage) }} · {{ system.cpu_cores }} {{ t('dashboard.system.cpuCores') }}</span>
              </div>
              <div class="h-2 overflow-hidden rounded-full bg-line">
                <div class="h-full rounded-full bg-accent" :style="{ width: formatPercent(system.cpu_usage) }" />
              </div>
            </div>
            <div>
              <div class="mb-1 flex justify-between text-xs text-muted">
                <span>{{ t('dashboard.system.memoryUsage') }}</span>
                <span class="tabular">{{ formatPercent(system.memory_usage) }} · {{ formatBytes(system.used_memory) }} / {{ formatBytes(system.total_memory) }}</span>
              </div>
              <div class="h-2 overflow-hidden rounded-full bg-line">
                <div class="h-full rounded-full bg-[var(--warning)]" :style="{ width: formatPercent(system.memory_usage) }" />
              </div>
            </div>
            <dl class="grid grid-cols-1 gap-2 text-sm sm:grid-cols-2">
              <div>
                <dt class="text-faint">{{ t('dashboard.system.hostName') }}</dt>
                <dd class="mono text-[13px]">{{ system.host_name || '—' }}</dd>
              </div>
              <div>
                <dt class="text-faint">{{ t('dashboard.system.osName') }}</dt>
                <dd>{{ system.os_name || '—' }}</dd>
              </div>
              <div>
                <dt class="text-faint">{{ t('dashboard.system.kernelVersion') }}</dt>
                <dd class="mono text-[13px]">{{ system.kernel_version || '—' }}</dd>
              </div>
              <div>
                <dt class="text-faint">{{ t('dashboard.system.uptime') }}</dt>
                <dd class="tabular">{{ formatDuration(system.uptime_secs) }}</dd>
              </div>
            </dl>
          </div>
          <p v-else-if="loading" class="mt-4 text-sm text-muted">{{ t('common.loading') }}</p>
          <p v-else class="mt-4 text-sm text-muted">{{ t('dashboard.emptyHost') }}</p>
        </section>

        <section class="ui-card p-5 xl:col-span-3">
          <div class="flex flex-wrap items-center justify-between gap-2">
            <h2 class="text-sm font-semibold">{{ t('dashboard.serverConfig') }}</h2>
            <div class="flex flex-wrap gap-1.5">
              <StatusBadge
                v-for="flag in flags"
                :key="flag.label"
                :label="`${flag.label}: ${flag.enabled ? t('common.on') : t('common.off')}`"
                :tone="flag.enabled ? 'success' : 'neutral'"
              />
            </div>
          </div>
          <dl class="mt-4 divide-y divide-line">
            <div v-for="row in configRows" :key="row[0]" class="flex flex-col gap-1 py-2 sm:flex-row sm:justify-between">
              <dt class="text-sm text-muted">{{ row[0] }}</dt>
              <dd class="mono break-all text-[13px] text-ink">{{ row[1] || '—' }}</dd>
            </div>
          </dl>
        </section>
      </div>
    </template>
  </div>
</template>
