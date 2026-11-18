<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { tunnelApi } from '@/api'
import { useAuthStore } from '@/stores/auth'
import type { Tunnel, TunnelListRequest, TunnelListenStatus } from '@/types'
import { accountScope, loadFilters, saveFilters } from '@/utils/filters'
import { clampQuery, isApiErrorPayload, supportsListQuery } from '@/utils/listQuery'
import { messageFromResult, type ApiResultLike } from '@/utils/apiMessage'
import { canManageTunnel, TUNNEL_TYPE_NAMES, tunnelDisplayName } from '@/utils/tunnel'
import { useTunnelMutations } from '@/composables/useTunnelMutations'
import PageHeader from '@/components/PageHeader.vue'
import StatusBadge from '@/components/StatusBadge.vue'
import Button from '@/components/ui/Button.vue'
import Input from '@/components/ui/Input.vue'
import Select from '@/components/ui/Select.vue'
import Segmented from '@/components/ui/Segmented.vue'
import Switch from '@/components/ui/Switch.vue'
import ConfirmDialog from '@/components/ui/ConfirmDialog.vue'
import EmptyState from '@/components/ui/EmptyState.vue'
import Spinner from '@/components/ui/Spinner.vue'
import Pagination from '@/components/ui/Pagination.vue'
import ActionMenu from '@/components/ui/ActionMenu.vue'
import CopyableText from '@/components/CopyableText.vue'
import TunnelEditorDrawer from '@/components/TunnelEditorDrawer.vue'
import DiagnoseDialog from '@/components/DiagnoseDialog.vue'
import TunnelReadonlyDrawer from '@/components/TunnelReadonlyDrawer.vue'

const { t } = useI18n()
const authStore = useAuthStore()
const editor = ref<InstanceType<typeof TunnelEditorDrawer> | null>(null)
const diagnose = ref<InstanceType<typeof DiagnoseDialog> | null>(null)

const loading = ref(false)
const loadError = ref('')
const tunnels = ref<Tunnel[]>([])
const searchText = ref('')
const statusFilter = ref<'all' | 'listening' | 'failed' | 'offline' | 'disabled'>('all')
const typeFilter = ref<'all' | '0' | '1' | '2' | '3'>('all')
const serverQuery = ref(false)
let loadSeq = 0
let searchTimer: ReturnType<typeof setTimeout> | undefined
const pagination = reactive({ currentPage: 1, pageSize: 20, total: 0 })
let persistReady = false
const detail = ref<InstanceType<typeof TunnelReadonlyDrawer> | null>(null)

const auth = () => ({ isAdmin: authStore.isAdmin, currentUserId: authStore.currentUserId })
const { toggling, confirm, requestToggle, requestRemove, confirmAction } = useTunnelMutations(() => loadData(pagination.currentPage))

const displayedTunnels = computed(() => {
  if (serverQuery.value) return tunnels.value
  return tunnels.value.filter((item) => {
    if (statusFilter.value !== 'all') {
      if (statusFilter.value === 'disabled' && item.enabled) return false
      if (statusFilter.value === 'listening' && item.listen_status !== 'listening') return false
      if (statusFilter.value === 'failed' && item.listen_status !== 'failed') return false
      if (statusFilter.value === 'offline' && item.listen_status !== 'offline') return false
    }
    if (typeFilter.value !== 'all' && String(item.tunnel_type) !== typeFilter.value) return false
    if (!searchText.value) return true
    const q = searchText.value.toLowerCase()
    return item.source.toLowerCase().includes(q)
      || item.endpoint.toLowerCase().includes(q)
      || item.description.toLowerCase().includes(q)
      || (item.sender_name || '').toLowerCase().includes(q)
      || (item.receiver_name || '').toLowerCase().includes(q)
      || String(item.id).includes(q)
      || String(item.sender).includes(q)
      || String(item.receiver).includes(q)
  })
})

const filterOptions = computed(() => [
  { label: t('common.all'), value: 'all' },
  { label: t('tunnel.filterListening'), value: 'listening' },
  { label: t('tunnel.filterFailed'), value: 'failed' },
  { label: t('tunnel.filterOffline'), value: 'offline' },
  { label: t('tunnel.filterDisabled'), value: 'disabled' },
])

const typeOptions = computed(() => [
  { label: t('common.all'), value: 'all' },
  { label: 'TCP', value: '0' },
  { label: 'UDP', value: '1' },
  { label: 'SOCKS5', value: '2' },
  { label: 'HTTP', value: '3' },
])

function restoreFilters() {
  const saved = loadFilters<{
    search?: string
    status?: typeof statusFilter.value
    type?: typeof typeFilter.value
    page?: number
    pageSize?: number
  }>('tunnels', accountScope(authStore.role, authStore.currentUserId))
  if (!saved) return 1
  if (saved.search != null) searchText.value = saved.search
  if (saved.status) statusFilter.value = saved.status
  if (saved.type) typeFilter.value = saved.type
  if (saved.pageSize) pagination.pageSize = saved.pageSize
  return saved.page || 1
}

watch([searchText, statusFilter, typeFilter, () => pagination.currentPage, () => pagination.pageSize], () => {
  if (!persistReady) return
  saveFilters('tunnels', accountScope(authStore.role, authStore.currentUserId), {
    search: searchText.value,
    status: statusFilter.value,
    type: typeFilter.value,
    page: pagination.currentPage,
    pageSize: pagination.pageSize,
  })
})

function buildTunnelRequest(page: number): TunnelListRequest {
  const req: TunnelListRequest = { page_number: page - 1, page_size: pagination.pageSize }
  const q = clampQuery(searchText.value)
  if (q) req.q = q
  if (typeFilter.value !== 'all') req.tunnel_type = Number(typeFilter.value)
  if (statusFilter.value !== 'all') req.listen_status = statusFilter.value as TunnelListenStatus
  return req
}

async function loadData(page = 1) {
  const seq = ++loadSeq
  loading.value = true
  loadError.value = ''
  pagination.currentPage = page
  try {
    const res = await tunnelApi.list(buildTunnelRequest(page))
    if (seq !== loadSeq) return
    if (isApiErrorPayload(res.data)) {
      loadError.value = messageFromResult(res.data as ApiResultLike, t, t('common.loadFailed'))
      tunnels.value = []
      pagination.total = 0
      serverQuery.value = false
      return
    }
    serverQuery.value = supportsListQuery(res.data)
    tunnels.value = res.data.tunnels ?? []
    pagination.total = res.data.total_count ?? 0
  } catch {
    if (seq !== loadSeq) return
    loadError.value = t('common.loadFailed')
  } finally {
    if (seq === loadSeq) loading.value = false
  }
}

function scheduleSearch() {
  if (!persistReady) return
  if (searchTimer) clearTimeout(searchTimer)
  searchTimer = setTimeout(() => { loadData(1) }, 300)
}

watch([statusFilter, typeFilter], () => {
  if (persistReady) loadData(1)
})

onBeforeUnmount(() => {
  if (searchTimer) clearTimeout(searchTimer)
})

function runtimeTone(tunnel: Tunnel) {
  if (!tunnel.enabled) return 'neutral' as const
  if (tunnel.listen_status === 'failed') return 'danger' as const
  if (tunnel.listen_status === 'offline') return 'warning' as const
  return tunnel.available ? 'success' as const : 'warning' as const
}
function runtimeLabel(tunnel: Tunnel) {
  if (!tunnel.enabled) return t('tunnel.runtime.disabled')
  if (tunnel.listen_status === 'failed') return t('tunnel.listen.failed')
  if (tunnel.listen_status === 'offline') return t('tunnel.listen.offline')
  if (tunnel.available) return t('tunnel.runtime.available')
  return t('tunnel.runtime.waiting')
}
function listenTip(tunnel: Tunnel) {
  if (tunnel.listen_status === 'failed') return tunnel.listen_error || ''
  if (tunnel.listen_status === 'unknown') return t('tunnel.listen.unknownTip')
  return ''
}
function endpointName(id: number, name: string) {
  if (id === 0) return t('common.server')
  return name || `#${id}`
}

function onMenu(tunnel: Tunnel, key: string) {
  if (key === 'clone') editor.value?.openClone(tunnel)
  else if (key === 'diagnose') diagnose.value?.run(tunnel)
  else if (key === 'delete') requestRemove(tunnel)
}

onMounted(async () => {
  const page = restoreFilters()
  persistReady = true
  await loadData(page)
})
</script>

<template>
  <div>
    <PageHeader :title="t('tunnel.title')" :subtitle="t('tunnel.subtitle')">
      <Button icon="refresh" :loading="loading" @click="loadData(pagination.currentPage)">{{ t('common.refresh') }}</Button>
      <Button variant="primary" icon="plus" @click="editor?.openAdd()">{{ t('tunnel.add') }}</Button>
    </PageHeader>

    <div class="ui-card overflow-hidden">
      <div class="flex flex-wrap items-center justify-between gap-3 border-b border-line px-4 py-3">
        <div class="flex min-w-0 flex-1 flex-wrap items-center gap-2">
          <Segmented :model-value="statusFilter" :options="filterOptions" @update:model-value="statusFilter = $event as typeof statusFilter" />
          <div class="w-32">
            <Select :model-value="typeFilter" @update:model-value="typeFilter = String($event) as typeof typeFilter">
              <option v-for="opt in typeOptions" :key="opt.value" :value="opt.value">{{ opt.label }}</option>
            </Select>
          </div>
        </div>
        <div class="w-full max-w-xs">
          <Input :model-value="searchText" :placeholder="t('tunnel.searchPlaceholder')" @update:model-value="searchText = String($event); scheduleSearch()" />
        </div>
      </div>
      <p class="border-b border-line px-4 py-2 text-xs text-faint">{{ serverQuery ? t('common.serverFilterActive') : t('common.currentPageFilterHint') }}</p>

      <Spinner v-if="loading && !tunnels.length" :label="t('common.loading')" />
      <EmptyState v-else-if="loadError" :title="t('common.loadFailed')" :description="loadError" icon="warning" retry @retry="loadData(pagination.currentPage)" />
      <EmptyState v-else-if="!displayedTunnels.length" :title="searchText || statusFilter !== 'all' || typeFilter !== 'all' ? t('common.noResults') : t('tunnel.empty')" icon="tunnel" />

      <div v-else>
        <div class="divide-y divide-line md:hidden">
          <article v-for="item in displayedTunnels" :key="item.id" class="space-y-3 px-4 py-4">
            <div class="flex items-start justify-between gap-3">
              <div class="min-w-0">
                <button type="button" class="truncate text-left font-medium text-ink hover:text-accent" @click="detail?.show(item)">{{ tunnelDisplayName(item) }}</button>
                <p class="mt-0.5 text-xs text-faint">{{ TUNNEL_TYPE_NAMES[item.tunnel_type] }} · {{ endpointName(item.receiver, item.receiver_name) }} ← {{ endpointName(item.sender, item.sender_name) }}</p>
              </div>
              <StatusBadge :tone="runtimeTone(item)" :label="runtimeLabel(item)" :title="listenTip(item)" />
            </div>
            <div class="space-y-1 text-sm">
              <div class="flex gap-2"><span class="shrink-0 text-faint">{{ t('tunnel.source') }}</span><CopyableText :value="item.source" /></div>
              <div class="flex gap-2"><span class="shrink-0 text-faint">{{ t('tunnel.endpoint') }}</span><CopyableText :value="item.endpoint" /></div>
            </div>
            <div v-if="canManageTunnel(item, auth())" class="flex items-center justify-between gap-2">
              <Switch :model-value="item.enabled" :label="item.enabled ? t('common.enabled') : t('common.disabled')" :disabled="toggling.has(item.id)" @update:model-value="requestToggle(item)" />
              <div class="flex items-center gap-1">
                <Button size="sm" variant="ghost" @click="editor?.openEdit(item)">{{ t('tunnel.edit') }}</Button>
                <ActionMenu
                  :items="[
                    { key: 'clone', label: t('tunnel.clone'), icon: 'copy' },
                    { key: 'diagnose', label: t('tunnel.diagnose'), icon: 'activity' },
                    { key: 'delete', label: t('common.delete'), icon: 'trash', tone: 'danger' },
                  ]"
                  @select="onMenu(item, $event)"
                />
              </div>
            </div>
            <div v-else class="flex justify-end">
              <Button size="sm" variant="ghost" icon="activity" @click="diagnose?.run(item)">{{ t('tunnel.diagnose') }}</Button>
            </div>
          </article>
        </div>

        <div class="hidden overflow-x-auto md:block">
          <table class="min-w-full text-left text-sm">
            <thead class="border-b border-line bg-canvas text-xs uppercase tracking-wide text-faint">
              <tr>
                <th class="px-4 py-3 font-medium">{{ t('tunnel.table.name') }}</th>
                <th class="px-4 py-3 font-medium">{{ t('common.status') }}</th>
                <th class="px-4 py-3 font-medium">{{ t('tunnel.source') }} / {{ t('tunnel.endpoint') }}</th>
                <th class="px-4 py-3 font-medium">{{ t('tunnel.enabled') }}</th>
                <th class="px-4 py-3 font-medium">{{ t('common.actions') }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="item in displayedTunnels" :key="item.id" class="border-b border-line last:border-0 hover:bg-canvas/80">
                <td class="px-4 py-3.5">
                  <button type="button" class="truncate text-left font-medium text-ink hover:text-accent" :title="tunnelDisplayName(item)" @click="detail?.show(item)">{{ tunnelDisplayName(item) }}</button>
                  <p class="mt-0.5 text-xs text-faint">{{ TUNNEL_TYPE_NAMES[item.tunnel_type] }} · {{ endpointName(item.receiver, item.receiver_name) }} ← {{ endpointName(item.sender, item.sender_name) }}</p>
                </td>
                <td class="px-4 py-3.5">
                  <StatusBadge :tone="runtimeTone(item)" :label="runtimeLabel(item)" />
                  <p v-if="listenTip(item)" class="mt-1 max-w-[14rem] truncate text-[11px] text-faint" :title="listenTip(item)">{{ listenTip(item) }}</p>
                </td>
                <td class="px-4 py-3.5">
                  <div class="max-w-[16rem] space-y-1">
                    <CopyableText :value="item.source" />
                    <CopyableText :value="item.endpoint" />
                  </div>
                </td>
                <td class="px-4 py-3.5">
                  <Switch
                    v-if="canManageTunnel(item, auth())"
                    :model-value="item.enabled"
                    :disabled="toggling.has(item.id)"
                    @update:model-value="requestToggle(item)"
                  />
                  <span v-else class="text-xs text-faint">{{ item.enabled ? t('common.enabled') : t('common.disabled') }}</span>
                </td>
                <td class="px-4 py-3.5">
                  <div v-if="canManageTunnel(item, auth())" class="flex items-center gap-1">
                    <Button size="sm" variant="ghost" @click="editor?.openEdit(item)">{{ t('tunnel.edit') }}</Button>
                    <ActionMenu
                      :items="[
                        { key: 'clone', label: t('tunnel.clone'), icon: 'copy' },
                        { key: 'diagnose', label: t('tunnel.diagnose'), icon: 'activity' },
                        { key: 'delete', label: t('common.delete'), icon: 'trash', tone: 'danger' },
                      ]"
                      @select="onMenu(item, $event)"
                    />
                  </div>
                  <Button v-else size="sm" variant="ghost" icon="activity" @click="diagnose?.run(item)">{{ t('tunnel.diagnose') }}</Button>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <Pagination
        v-if="pagination.total > 0"
        class="border-t border-line px-4"
        :page="pagination.currentPage"
        :page-size="pagination.pageSize"
        :total="pagination.total"
        @update:page="loadData($event)"
        @update:page-size="pagination.pageSize = $event; loadData(1)"
      />
    </div>

    <TunnelEditorDrawer ref="editor" @saved="loadData(pagination.currentPage)" />
    <TunnelReadonlyDrawer ref="detail" @edit="editor?.openEdit($event)" @diagnose="diagnose?.run($event)" />
    <DiagnoseDialog ref="diagnose" />
    <ConfirmDialog
      :open="confirm.open"
      :title="confirm.title"
      :message="confirm.message"
      :details="confirm.details"
      :loading="confirm.loading"
      :confirm-text="confirm.confirmText"
      :tone="confirm.tone"
      @update:open="confirm.open = $event"
      @confirm="confirmAction"
    />
  </div>
</template>
