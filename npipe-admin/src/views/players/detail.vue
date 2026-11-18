<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import VChart from 'vue-echarts'
import { use } from 'echarts/core'
import { CanvasRenderer } from 'echarts/renderers'
import { LineChart } from 'echarts/charts'
import { GridComponent, LegendComponent, TooltipComponent } from 'echarts/components'
import { playerApi, tunnelApi } from '@/api'
import { useAuthStore } from '@/stores/auth'
import { useAppStore } from '@/stores/app'
import { useToastStore } from '@/stores/toast'
import type { PlayerDetail, PlayerTunnelItem, TrafficStatsResponse, Tunnel } from '@/types'
import { formatBytes, formatDuration, formatHourKey, formatHourLabel } from '@/utils/format'
import { formatDateTime } from '@/utils/time'
import { canManageTunnel, TUNNEL_TYPE_NAMES, tunnelDisplayName } from '@/utils/tunnel'
import { isApiErrorPayload, resolveAssociatedTunnelSource, supportsListQuery } from '@/utils/listQuery'
import { messageFromResult, messageFromUnknown } from '@/utils/apiMessage'
import { useUnsavedGuard } from '@/composables/useUnsavedGuard'
import { useTunnelMutations } from '@/composables/useTunnelMutations'
import PageHeader from '@/components/PageHeader.vue'
import StatusBadge from '@/components/StatusBadge.vue'
import Button from '@/components/ui/Button.vue'
import Field from '@/components/ui/Field.vue'
import Input from '@/components/ui/Input.vue'
import Dialog from '@/components/ui/Dialog.vue'
import ConfirmDialog from '@/components/ui/ConfirmDialog.vue'
import EmptyState from '@/components/ui/EmptyState.vue'
import Spinner from '@/components/ui/Spinner.vue'
import Segmented from '@/components/ui/Segmented.vue'
import Switch from '@/components/ui/Switch.vue'
import ActionMenu from '@/components/ui/ActionMenu.vue'
import Pagination from '@/components/ui/Pagination.vue'
import CopyableText from '@/components/CopyableText.vue'
import TunnelEditorDrawer from '@/components/TunnelEditorDrawer.vue'
import TunnelReadonlyDrawer from '@/components/TunnelReadonlyDrawer.vue'
import DiagnoseDialog from '@/components/DiagnoseDialog.vue'

use([CanvasRenderer, LineChart, GridComponent, LegendComponent, TooltipComponent])

const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const authStore = useAuthStore()
const appStore = useAppStore()
const toast = useToastStore()

const loading = ref(false)
const loadError = ref('')
const trafficLoading = ref(false)
const player = ref<PlayerDetail | null>(null)
const trafficStats = ref<TrafficStatsResponse | null>(null)
const trafficHours = ref(24)

const playerId = computed(() => Number(route.params.id))
const trafficRangeOptions = computed(() => [
  { label: t('player.trafficRange24h'), value: 24 },
  { label: t('player.trafficRange72h'), value: 72 },
  { label: t('player.trafficRange7d'), value: 168 },
  { label: t('player.trafficRange30d'), value: 720 },
])

const trafficChartItems = computed(() => {
  const byHour = new Map((trafficStats.value?.items ?? []).map((item) => [item.hour, item]))
  const now = new Date()
  now.setUTCMinutes(0, 0, 0)
  return Array.from({ length: trafficHours.value }, (_, index) => {
    const hour = new Date(now.getTime() - (trafficHours.value - index - 1) * 3600 * 1000)
    const key = formatHourKey(hour)
    const item = byHour.get(key)
    return {
      hour: key,
      label: formatHourLabel(key, trafficHours.value <= 24),
      bytes_in: item?.bytes_in ?? 0,
      bytes_out: item?.bytes_out ?? 0,
    }
  })
})

const trafficChartOption = computed(() => {
  const items = trafficChartItems.value
  const isDark = appStore.resolvedTheme === 'dark'
  const axisColor = isDark ? '#9AA5B1' : '#5C6770'
  const gridColor = isDark ? 'rgba(255,255,255,.06)' : 'rgba(16,21,28,.08)'
  const tooltipBg = isDark ? 'rgba(18,24,31,.96)' : 'rgba(255,255,255,.96)'
  const tooltipText = isDark ? '#E8EEF3' : '#12171C'
  return {
    backgroundColor: 'transparent',
    color: ['#2EE6A6', '#6AA0FF'],
    tooltip: {
      trigger: 'axis',
      backgroundColor: tooltipBg,
      borderColor: 'rgba(46,230,166,.28)',
      textStyle: { color: tooltipText },
      valueFormatter: (value: number) => formatBytes(value),
    },
    legend: {
      top: 4,
      right: 0,
      textStyle: { color: axisColor },
      data: [t('player.trafficIn'), t('player.trafficOut')],
    },
    grid: { left: 10, right: 14, top: 42, bottom: 10, containLabel: true },
    xAxis: {
      type: 'category',
      boundaryGap: false,
      data: items.map((item) => item.label),
      axisLabel: { color: axisColor, hideOverlap: true },
      axisLine: { lineStyle: { color: gridColor } },
      axisTick: { lineStyle: { color: gridColor } },
    },
    yAxis: {
      type: 'value',
      axisLabel: { color: axisColor, formatter: (value: number) => formatBytes(value) },
      splitLine: { lineStyle: { color: gridColor } },
    },
    series: [
      {
        name: t('player.trafficIn'),
        type: 'line',
        smooth: true,
        showSymbol: false,
        lineStyle: { width: 3 },
        areaStyle: { color: 'rgba(46,230,166,.18)' },
        data: items.map((item) => item.bytes_in),
      },
      {
        name: t('player.trafficOut'),
        type: 'line',
        smooth: true,
        showSymbol: false,
        lineStyle: { width: 2 },
        areaStyle: { color: 'rgba(106,160,255,.14)' },
        data: items.map((item) => item.bytes_out),
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
  loadError.value = ''
  trafficStats.value = null
  try {
    const res = await playerApi.detail({ id: playerId.value })
    player.value = res.data.player
    if (!player.value) {
      toast.error(t('player.notFound'))
      return
    }
    loadTrafficStats()
    loadAssociatedTunnels(associatedLimited.value ? 1 : associatedPage.currentPage)
  } catch {
    loadError.value = t('common.loadFailed')
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

watch(trafficHours, loadTrafficStats)

const editor = ref<InstanceType<typeof TunnelEditorDrawer> | null>(null)
const diagnoseRef = ref<InstanceType<typeof DiagnoseDialog> | null>(null)
const detailDrawer = ref<InstanceType<typeof TunnelReadonlyDrawer> | null>(null)
const associatedTunnels = ref<Tunnel[]>([])
const associatedLegacy = ref<PlayerTunnelItem[]>([])
const associatedLimited = ref(false)
const associatedLoading = ref(false)
let associatedSeq = 0
const associatedPage = reactive({ currentPage: 1, pageSize: 20, total: 0 })
const { toggling, confirm: tunnelConfirm, requestToggle, requestRemove, confirmAction: confirmTunnelAction } = useTunnelMutations(() => {
  loadDetail()
})

function associatedRole(item: Tunnel): string {
  const uid = player.value?.id
  if (uid == null) return 'receiver'
  const sender = item.sender === uid
  const receiver = item.receiver === uid
  if (sender && receiver) return 'both'
  if (sender) return 'sender'
  return 'receiver'
}

function canManageAssociatedTunnel(item: Tunnel) {
  return canManageTunnel(item, { isAdmin: authStore.isAdmin, currentUserId: authStore.currentUserId })
}

function canManageLegacy(item: PlayerTunnelItem) {
  // On an old server, receiver-only rows must not be treated as editable.
  return authStore.isAdmin || (item.role === 'sender' || item.role === 'both')
}

async function loadAssociatedTunnels(page = 1) {
  const userId = player.value?.id
  if (!userId) return
  const seq = ++associatedSeq
  associatedLoading.value = true
  try {
    const res = await tunnelApi.list({
      page_number: page - 1,
      page_size: associatedPage.pageSize,
      user_id: userId,
      user_role: 'any',
    })
    if (seq !== associatedSeq) return
    if (isApiErrorPayload(res.data) || resolveAssociatedTunnelSource(res.data) !== 'server' || !supportsListQuery(res.data)) {
      // Old servers ignore user_id. Discard the list and keep the embedded 20-item profile list.
      associatedLimited.value = true
      associatedTunnels.value = []
      associatedLegacy.value = player.value?.tunnels ?? []
      associatedPage.total = associatedLegacy.value.length
      associatedPage.currentPage = 1
      return
    }
    associatedLimited.value = false
    associatedLegacy.value = []
    associatedTunnels.value = res.data.tunnels ?? []
    associatedPage.total = res.data.total_count ?? 0
    associatedPage.currentPage = page
  } catch {
    if (seq !== associatedSeq) return
    associatedLimited.value = true
    associatedTunnels.value = []
    associatedLegacy.value = player.value?.tunnels ?? []
    associatedPage.total = associatedLegacy.value.length
    associatedPage.currentPage = 1
  } finally {
    if (seq === associatedSeq) associatedLoading.value = false
  }
}

function onAssociatedMenu(item: Tunnel, key: string) {
  if (!canManageAssociatedTunnel(item)) {
    toast.error(t('common.forbidden'))
    return
  }
  if (key === 'edit') editor.value?.openEdit(item)
  else if (key === 'diagnose') diagnoseRef.value?.run(item)
  else if (key === 'delete') requestRemove(item)
}

function onAssociatedToggle(item: Tunnel) {
  if (!canManageAssociatedTunnel(item)) {
    toast.error(t('common.forbidden'))
    return
  }
  requestToggle(item)
}

function onLegacyMenu(item: PlayerTunnelItem, key: string) {
  if (!canManageLegacy(item)) {
    toast.error(t('common.forbidden'))
    return
  }
  // Edit still goes through tunnel_detail, which enforces manage permission on the server.
  if (key === 'edit') editor.value?.openEdit({ id: item.id })
  else if (key === 'diagnose') {
    diagnoseRef.value?.run({
      id: item.id,
      source: item.source,
      endpoint: item.endpoint,
      sender: 0,
      receiver: 0,
      tunnel_type: item.tunnel_type as 0 | 1 | 2 | 3,
    })
  } else if (key === 'delete') {
    requestRemove({
      id: item.id,
      source: item.source,
      endpoint: item.endpoint,
      enabled: item.enabled,
      sender: authStore.currentUserId,
      receiver: 0,
      sender_name: '',
      receiver_name: '',
      description: '',
      tunnel_type: item.tunnel_type as 0 | 1 | 2 | 3,
      username: '',
      is_compressed: false,
      encryption_method: 'None',
      custom_mapping: {},
      sender_online: false,
      receiver_online: false,
      available: item.available,
    })
  }
}

function onLegacyToggle(item: PlayerTunnelItem) {
  if (!canManageLegacy(item)) {
    toast.error(t('common.forbidden'))
    return
  }
  requestToggle({
    id: item.id,
    source: item.source,
    endpoint: item.endpoint,
    enabled: item.enabled,
    sender: authStore.currentUserId,
    receiver: 0,
    sender_name: '',
    receiver_name: '',
    description: '',
    tunnel_type: item.tunnel_type as 0 | 1 | 2 | 3,
    username: '',
    is_compressed: false,
    encryption_method: 'None',
    custom_mapping: {},
    sender_online: false,
    receiver_online: false,
    available: item.available,
  })
}

function openCreateTunnel() {
  if (!player.value) return
  if (!authStore.isAdmin && player.value.id !== authStore.currentUserId) return
  editor.value?.openAddForUser(player.value.id)
}

function viewLoginLogs() {
  if (!player.value) return
  router.push({ name: 'LoginLogs', query: { user_id: String(player.value.id) } })
}

const renameOpen = ref(false)
const passwordOpen = ref(false)
const renameLoading = ref(false)
const passwordLoading = ref(false)
const renameForm = reactive({ id: 0, username: '' })
const passwordForm = reactive({ id: 0, password: '' })
const renameError = ref('')
const passwordError = ref('')
const renameDirty = computed(() => renameOpen.value && renameForm.username !== (player.value?.username ?? ''))
const passwordDirty = computed(() => passwordOpen.value && !!passwordForm.password)
const unsaved = computed(() => renameDirty.value || passwordDirty.value)
const { confirmDiscard } = useUnsavedGuard(unsaved, () => t('common.unsavedLeave'))
function onRenameOpen(next: boolean) {
  if (!next && !confirmDiscard()) return
  renameOpen.value = next
}
function onPasswordOpen(next: boolean) {
  if (!next && !confirmDiscard()) return
  passwordOpen.value = next
}

function openRename() {
  if (!player.value || !authStore.isAdmin) return
  renameForm.id = player.value.id
  renameForm.username = player.value.username
  renameError.value = ''
  renameOpen.value = true
}

function openPassword() {
  if (!player.value) return
  passwordForm.id = player.value.id
  passwordForm.password = ''
  passwordError.value = ''
  passwordOpen.value = true
}

async function handleRename() {
  if (renameLoading.value) return
  renameError.value = !renameForm.username.trim()
    ? t('player.validation.usernameRequired')
    : (renameForm.username.length > 30 ? t('player.validation.username') : '')
  if (renameError.value) return
  renameLoading.value = true
  try {
    const res = await playerApi.rename({ ...renameForm })
    if (res.data.code === 0) {
      toast.success(t('player.saveSuccess'))
      renameOpen.value = false
      loadDetail()
    } else {
      toast.error(messageFromResult(res.data, t, t('common.failed')))
    }
  } finally {
    renameLoading.value = false
  }
}

const selfResetConfirm = ref(false)

async function actuallyResetPassword() {
  if (passwordLoading.value) return
  passwordLoading.value = true
  try {
    const res = await playerApi.resetPassword({ ...passwordForm })
    if (res.data.code === 0) {
      toast.success(t('player.passwordResetSuccess'))
      passwordOpen.value = false
      selfResetConfirm.value = false
      if (authStore.isAdmin) {
        loadDetail()
      } else {
        authStore.clearSession()
        router.replace('/login')
      }
    } else {
      toast.error(messageFromResult(res.data, t, t('common.failed')))
    }
  } finally {
    passwordLoading.value = false
  }
}

async function handleResetPassword() {
  if (passwordLoading.value) return
  passwordError.value = !passwordForm.password
    ? t('player.validation.passwordRequired')
    : (passwordForm.password.length > 15 ? t('player.validation.password') : '')
  if (passwordError.value) return
  selfResetConfirm.value = true
}

type DetailAction = 'status' | 'webAccess' | 'kick'
const action = reactive({
  open: false,
  loading: false,
  kind: '' as DetailAction | '',
  title: '',
  message: '',
  confirmText: '',
  tone: 'warning' as 'warning' | 'success' | 'danger',
  details: [] as { label: string; value: string | number }[],
})

function playerDetails() {
  if (!player.value) return []
  return [
    { label: t('common.id'), value: player.value.id },
    { label: t('player.username'), value: player.value.username },
    { label: t('player.table.status'), value: player.value.online ? t('common.online') : t('common.offline') },
  ]
}

function handleToggleStatus() {
  if (!player.value || !authStore.isAdmin) return
  action.kind = 'status'
  action.title = t(player.value.enabled ? 'player.disableTitle' : 'player.enableTitle')
  action.message = t(player.value.enabled ? 'player.disableConfirm' : 'player.enableConfirm', { name: player.value.username })
  action.confirmText = t(player.value.enabled ? 'common.disable' : 'common.enable')
  action.tone = player.value.enabled ? 'warning' : 'success'
  action.details = playerDetails()
  action.loading = false
  action.open = true
}

function handleToggleWebAccess() {
  if (!player.value || !authStore.isAdmin) return
  action.kind = 'webAccess'
  action.title = t(player.value.web_access ? 'player.revokeWebAccessTitle' : 'player.grantWebAccessTitle')
  action.message = t(player.value.web_access ? 'player.revokeWebAccessConfirm' : 'player.grantWebAccessConfirm', { name: player.value.username })
  action.confirmText = t('common.confirm')
  action.tone = player.value.web_access ? 'warning' : 'success'
  action.details = playerDetails()
  action.loading = false
  action.open = true
}

function handleKick() {
  if (!player.value || !authStore.isAdmin) return
  action.kind = 'kick'
  action.title = t('player.kickTitle')
  action.message = t('player.kickConfirm', { name: player.value.username })
  action.confirmText = t('common.confirm')
  action.tone = 'warning'
  action.details = playerDetails()
  action.loading = false
  action.open = true
}

async function handleActionConfirm() {
  const target = player.value
  if (!target || !action.kind || action.loading) return
  action.loading = true
  try {
    if (action.kind === 'status') {
      const wasEnabled = target.enabled
      const res = await playerApi.updateStatus({ id: target.id, enabled: wasEnabled ? 0 : 1 })
      if (res.data.code === 0) {
        toast.success(wasEnabled ? t('player.disableSuccess') : t('player.enableSuccess'))
        action.open = false
        loadDetail()
      } else toast.error(messageFromResult(res.data, t, t('common.failed')))
      return
    }
    if (action.kind === 'webAccess') {
      const had = target.web_access
      const res = await playerApi.updateWebAccess({ id: target.id, web_access: had ? 0 : 1 })
      if (res.data.code === 0) {
        toast.success(had ? t('player.revokeWebAccessSuccess') : t('player.grantWebAccessSuccess'))
        action.open = false
        loadDetail()
      } else toast.error(messageFromResult(res.data, t, t('common.failed')))
      return
    }
    const res = await playerApi.kick({ id: target.id })
    if (res.data.code === 0) {
      toast.success(t('player.kickSuccess'))
      action.open = false
      loadDetail()
    } else toast.error(messageFromResult(res.data, t, t('common.failed')))
  } finally {
    action.loading = false
  }
}

const deleteOpen = ref(false)
const deleteLoading = ref(false)

function handleRemove() {
  if (!player.value || !authStore.isAdmin) return
  deleteOpen.value = true
}

async function handleRemoveConfirm() {
  if (!player.value || deleteLoading.value) return
  deleteLoading.value = true
  try {
    const res = await playerApi.remove({ id: player.value.id })
    if (res.data.code === 0) {
      toast.success(t('player.deleteSuccess'))
      deleteOpen.value = false
      router.push('/players')
    } else {
      toast.error(messageFromResult(res.data, t, t('common.failed')))
    }
  } finally {
    deleteLoading.value = false
  }
}

function tunnelRole(role: string) {
  if (role === 'sender') return t('player.tunnelRole.sender')
  if (role === 'receiver') return t('player.tunnelRole.receiver')
  return t('player.tunnelRole.both')
}

watch(() => route.params.id, () => {
  associatedTunnels.value = []
  associatedLegacy.value = []
  associatedLimited.value = false
  associatedPage.currentPage = 1
  loadDetail()
})
onMounted(loadDetail)
</script>

<template>
  <div>
    <PageHeader :title="player?.username || t('player.detailTitle')" :subtitle="t('player.subtitle')">
      <Button v-if="authStore.isAdmin" variant="secondary" icon="arrowLeft" @click="router.push('/players')">{{ t('common.back') }}</Button>
      <Button icon="refresh" :loading="loading" @click="loadDetail">{{ t('common.refresh') }}</Button>
      <Button v-if="player && (authStore.isAdmin || player.id === authStore.currentUserId)" variant="secondary" @click="viewLoginLogs">{{ t('player.viewLoginLogs') }}</Button>
      <Button v-if="player && (authStore.isAdmin || player.id === authStore.currentUserId)" variant="primary" icon="plus" @click="openCreateTunnel">{{ t('player.createTunnelForUser') }}</Button>
    </PageHeader>

    <Spinner v-if="loading && !player" :label="t('common.loading')" />
    <EmptyState v-else-if="loadError" :title="t('common.loadFailed')" :description="loadError" icon="warning" retry @retry="loadDetail" />
    <EmptyState v-else-if="!player" :title="t('player.notFound')" icon="user" />

    <template v-else>
      <section class="ui-card p-5">
        <div class="flex flex-wrap items-start justify-between gap-4">
          <div class="space-y-3">
            <div class="flex flex-wrap items-center gap-2">
              <StatusBadge :pulse="player.online" :tone="player.online ? 'success' : 'neutral'" :label="player.online ? t('common.online') : t('common.offline')" />
              <StatusBadge :tone="player.enabled ? 'success' : 'danger'" :label="player.enabled ? t('player.enabled') : t('player.disabled')" />
              <StatusBadge :tone="player.web_access ? 'accent' : 'neutral'" :label="player.web_access ? t('player.allowed') : t('player.notAllowed')" />
            </div>
            <dl class="grid grid-cols-2 gap-x-8 gap-y-2 text-sm sm:grid-cols-3">
              <div><dt class="text-faint">ID</dt><dd class="tabular">{{ player.id }}</dd></div>
              <div><dt class="text-faint">{{ t('player.createTime') }}</dt><dd>{{ formatDateTime(player.create_time) }}</dd></div>
              <div><dt class="text-faint">{{ t('player.table.ip') }}</dt><dd class="mono">{{ player.ip_addr || '—' }}</dd></div>
              <div><dt class="text-faint">{{ t('player.table.protocol') }}</dt><dd>{{ player.connection_protocol || '—' }}</dd></div>
              <div><dt class="text-faint">{{ t('player.table.onlineTime') }}</dt><dd>{{ player.online ? formatDateTime(player.online_time) : '—' }}</dd></div>
              <div><dt class="text-faint">{{ t('player.totalTransferTraffic') }}</dt><dd class="tabular">{{ formatBytes(player.total_bytes_in) }} / {{ formatBytes(player.total_bytes_out) }}</dd></div>
            </dl>
          </div>
          <div class="flex flex-wrap gap-2">
            <Button v-if="authStore.isAdmin" size="sm" @click="openRename">{{ t('player.rename') }}</Button>
            <Button size="sm" @click="openPassword">{{ t('player.resetPassword') }}</Button>
            <Button v-if="authStore.isAdmin" size="sm" @click="handleToggleStatus">{{ player.enabled ? t('player.disable') : t('player.enable') }}</Button>
            <Button v-if="authStore.isAdmin" size="sm" @click="handleToggleWebAccess">{{ player.web_access ? t('player.revokeWebAccess') : t('player.grantWebAccess') }}</Button>
            <Button v-if="authStore.isAdmin && player.online" size="sm" @click="handleKick">{{ t('player.kick') }}</Button>
            <Button v-if="authStore.isAdmin" size="sm" variant="danger" @click="handleRemove">{{ t('player.delete') }}</Button>
          </div>
        </div>
      </section>

      <section class="ui-card mt-4 p-5">
        <div class="mb-3 flex flex-wrap items-center justify-between gap-3">
          <h2 class="text-sm font-semibold">{{ t('player.trafficTrend') }}</h2>
          <Segmented :model-value="trafficHours" :options="trafficRangeOptions" @update:model-value="trafficHours = Number($event)" />
        </div>
        <div v-if="trafficLoading" class="py-16 text-center text-sm text-muted">{{ t('common.loading') }}</div>
        <div v-else-if="!(trafficStats?.items ?? []).some((i) => i.bytes_in || i.bytes_out)" class="py-10 text-center text-sm text-muted">{{ t('player.noTraffic') }}</div>
        <VChart v-else class="h-[280px] w-full" :option="trafficChartOption" autoresize />
      </section>

      <div class="mt-4 grid gap-4 xl:grid-cols-2">
        <section class="ui-card overflow-hidden">
          <div class="border-b border-line px-5 py-3 text-sm font-semibold">{{ t('player.associatedTunnels') }}</div>
          <p v-if="associatedLimited" class="border-b border-line px-5 py-2 text-xs text-faint">{{ t('common.associatedTunnelLimit') }}</p>
          <Spinner v-if="associatedLoading && !associatedTunnels.length && !associatedLegacy.length" :label="t('common.loading')" />
          <EmptyState v-else-if="!associatedLimited && !associatedTunnels.length" :title="t('player.noTunnels')" icon="tunnel" />
          <EmptyState v-else-if="associatedLimited && !associatedLegacy.length" :title="t('player.noTunnels')" icon="tunnel" />
          <div v-else class="overflow-x-auto">
            <table class="min-w-full text-left text-sm">
              <thead class="text-xs uppercase tracking-wide text-faint">
                <tr>
                  <th class="px-4 py-2 font-medium">{{ t('common.name') }}</th>
                  <th class="px-4 py-2 font-medium">{{ t('common.type') }}</th>
                  <th class="px-4 py-2 font-medium">{{ t('tunnel.source') }}</th>
                  <th class="px-4 py-2 font-medium">{{ t('tunnel.endpoint') }}</th>
                  <th class="px-4 py-2 font-medium">{{ t('common.status') }}</th>
                  <th class="px-4 py-2 font-medium">{{ t('common.actions') }}</th>
                </tr>
              </thead>
              <tbody v-if="!associatedLimited">
                <tr v-for="item in associatedTunnels" :key="item.id" class="border-t border-line">
                  <td class="px-4 py-3">
                    <button type="button" class="text-left font-medium text-ink hover:text-accent" @click="detailDrawer?.show(item)">{{ tunnelDisplayName(item) }}</button>
                  </td>
                  <td class="px-4 py-3">{{ TUNNEL_TYPE_NAMES[item.tunnel_type] ?? item.tunnel_type }} · {{ tunnelRole(associatedRole(item)) }}</td>
                  <td class="px-4 py-3"><CopyableText :value="item.source" /></td>
                  <td class="px-4 py-3"><CopyableText :value="item.endpoint" /></td>
                  <td class="px-4 py-3">
                    <StatusBadge :tone="!item.enabled ? 'neutral' : item.available ? 'success' : 'warning'" :label="!item.enabled ? t('tunnel.runtime.disabled') : item.available ? t('tunnel.runtime.available') : t('tunnel.runtime.waiting')" />
                  </td>
                  <td class="px-4 py-3">
                    <div v-if="canManageAssociatedTunnel(item)" class="flex items-center gap-1">
                      <Switch :model-value="item.enabled" :disabled="toggling.has(item.id)" @update:model-value="onAssociatedToggle(item)" />
                      <Button size="sm" variant="ghost" @click="onAssociatedMenu(item, 'edit')">{{ t('tunnel.edit') }}</Button>
                      <ActionMenu
                        :items="[
                          { key: 'diagnose', label: t('tunnel.diagnose'), icon: 'activity' },
                          { key: 'delete', label: t('common.delete'), icon: 'trash', tone: 'danger' },
                        ]"
                        @select="onAssociatedMenu(item, $event)"
                      />
                    </div>
                    <Button v-else size="sm" variant="ghost" icon="activity" @click="diagnoseRef?.run(item)">{{ t('tunnel.diagnose') }}</Button>
                  </td>
                </tr>
              </tbody>
              <tbody v-else>
                <tr v-for="item in associatedLegacy" :key="item.id" class="border-t border-line">
                  <td class="px-4 py-3 font-medium">#{{ item.id }}</td>
                  <td class="px-4 py-3">{{ TUNNEL_TYPE_NAMES[item.tunnel_type] ?? item.tunnel_type }} · {{ tunnelRole(item.role) }}</td>
                  <td class="px-4 py-3"><CopyableText :value="item.source" /></td>
                  <td class="px-4 py-3"><CopyableText :value="item.endpoint" /></td>
                  <td class="px-4 py-3">
                    <StatusBadge :tone="!item.enabled ? 'neutral' : item.available ? 'success' : 'warning'" :label="!item.enabled ? t('tunnel.runtime.disabled') : item.available ? t('tunnel.runtime.available') : t('tunnel.runtime.waiting')" />
                  </td>
                  <td class="px-4 py-3">
                    <div v-if="canManageLegacy(item)" class="flex items-center gap-1">
                      <Switch :model-value="item.enabled" :disabled="toggling.has(item.id)" @update:model-value="onLegacyToggle(item)" />
                      <Button size="sm" variant="ghost" @click="onLegacyMenu(item, 'edit')">{{ t('tunnel.edit') }}</Button>
                      <ActionMenu
                        :items="[
                          { key: 'diagnose', label: t('tunnel.diagnose'), icon: 'activity' },
                          { key: 'delete', label: t('common.delete'), icon: 'trash', tone: 'danger' },
                        ]"
                        @select="onLegacyMenu(item, $event)"
                      />
                    </div>
                  </td>
                </tr>
              </tbody>
            </table>
            <Pagination
              v-if="!associatedLimited && associatedPage.total > associatedPage.pageSize"
              class="border-t border-line px-4"
              :page="associatedPage.currentPage"
              :page-size="associatedPage.pageSize"
              :total="associatedPage.total"
              @update:page="loadAssociatedTunnels($event)"
              @update:page-size="associatedPage.pageSize = $event; loadAssociatedTunnels(1)"
            />
          </div>
        </section>

        <section class="ui-card overflow-hidden">
          <div class="border-b border-line px-5 py-3 text-sm font-semibold">{{ t('player.recentLogins') }}</div>
          <EmptyState v-if="!player.recent_logins.length" :title="t('player.noRecentLogins')" icon="log" />
          <div v-else class="overflow-x-auto">
            <table class="min-w-full text-left text-sm">
              <thead class="text-xs uppercase tracking-wide text-faint">
                <tr>
                  <th class="px-4 py-2 font-medium">{{ t('loginLog.loginTime') }}</th>
                  <th class="px-4 py-2 font-medium">{{ t('loginLog.source') }}</th>
                  <th class="px-4 py-2 font-medium">{{ t('loginLog.ip') }}</th>
                  <th class="px-4 py-2 font-medium">{{ t('loginLog.result') }}</th>
                  <th class="px-4 py-2 font-medium">{{ t('loginLog.duration') }}</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="item in player.recent_logins" :key="item.id" class="border-t border-line">
                  <td class="px-4 py-2 tabular">{{ formatDateTime(item.login_time) }}</td>
                  <td class="px-4 py-2">{{ item.login_source === 'web' ? t('loginLog.sourceWeb') : t('loginLog.sourceClient') }}</td>
                  <td class="mono px-4 py-2 text-[13px]">{{ item.ip_addr || '—' }}</td>
                  <td class="px-4 py-2">
                    <StatusBadge :tone="item.success ? 'success' : 'danger'" :label="item.success ? t('loginLog.success') : t('loginLog.failed')" />
                  </td>
                  <td class="px-4 py-2">{{ item.logout_time ? formatDuration(item.duration_secs) : t('loginLog.online') }}</td>
                </tr>
              </tbody>
            </table>
          </div>
        </section>
      </div>
    </template>

    <Dialog :open="renameOpen" :title="t('player.renameTitle')" @update:open="onRenameOpen">
      <Field :label="t('player.username')" required :error="renameError">
        <Input v-model="renameForm.username" :invalid="!!renameError" />
      </Field>
      <template #footer>
        <Button variant="secondary" :disabled="renameLoading" @click="onRenameOpen(false)">{{ t('common.cancel') }}</Button>
        <Button variant="primary" :loading="renameLoading" @click="handleRename">{{ t('common.save') }}</Button>
      </template>
    </Dialog>

    <Dialog :open="passwordOpen" :title="t('player.resetPasswordTitle')" @update:open="onPasswordOpen">
      <Field :label="t('player.newPassword')" required :error="passwordError">
        <Input v-model="passwordForm.password" type="password" autocomplete="new-password" :invalid="!!passwordError" />
      </Field>
      <template #footer>
        <Button variant="secondary" :disabled="passwordLoading" @click="onPasswordOpen(false)">{{ t('common.cancel') }}</Button>
        <Button variant="primary" :loading="passwordLoading" @click="handleResetPassword">{{ t('common.save') }}</Button>
      </template>
    </Dialog>

    <ConfirmDialog
      :open="selfResetConfirm"
      :title="t('player.resetPasswordTitle')"
      :message="!authStore.isAdmin || player?.id === authStore.currentUserId ? t('player.selfResetPasswordConfirm') : t('player.resetPasswordConfirm', { name: player?.username || '' })"
      :loading="passwordLoading"
      tone="warning"
      @update:open="selfResetConfirm = $event"
      @confirm="actuallyResetPassword"
    />

    <ConfirmDialog
      :open="action.open"
      :title="action.title"
      :message="action.message"
      :details="action.details"
      :loading="action.loading"
      :confirm-text="action.confirmText"
      :tone="action.tone"
      @update:open="action.open = $event"
      @confirm="handleActionConfirm"
    />

    <ConfirmDialog
      v-if="player"
      :open="deleteOpen"
      :title="t('player.deleteTitle')"
      :message="t('player.deleteConfirm', { name: player.username })"
      :details="playerDetails()"
      :loading="deleteLoading"
      :confirm-text="t('common.delete')"
      tone="danger"
      @update:open="deleteOpen = $event"
      @confirm="handleRemoveConfirm"
    />
    <TunnelEditorDrawer ref="editor" @saved="loadDetail" />
    <TunnelReadonlyDrawer ref="detailDrawer" @edit="editor?.openEdit($event)" @diagnose="diagnoseRef?.run($event)" />
    <DiagnoseDialog ref="diagnoseRef" />
    <ConfirmDialog
      :open="tunnelConfirm.open"
      :title="tunnelConfirm.title"
      :message="tunnelConfirm.message"
      :details="tunnelConfirm.details"
      :loading="tunnelConfirm.loading"
      :confirm-text="tunnelConfirm.confirmText"
      :tone="tunnelConfirm.tone"
      @update:open="tunnelConfirm.open = $event"
      @confirm="confirmTunnelAction"
    />
  </div>
</template>
