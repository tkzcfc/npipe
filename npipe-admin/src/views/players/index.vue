<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { playerApi } from '@/api'
import { useAuthStore } from '@/stores/auth'
import { useToastStore } from '@/stores/toast'
import type { Player, PlayerListRequest } from '@/types'
import { accountScope, loadFilters, saveFilters } from '@/utils/filters'
import { clampQuery, isApiErrorPayload, supportsListQuery } from '@/utils/listQuery'
import { messageFromResult, type ApiResultLike, messageFromUnknown } from '@/utils/apiMessage'
import { useUnsavedGuard } from '@/composables/useUnsavedGuard'
import PageHeader from '@/components/PageHeader.vue'
import StatusBadge from '@/components/StatusBadge.vue'
import Button from '@/components/ui/Button.vue'
import Input from '@/components/ui/Input.vue'
import Select from '@/components/ui/Select.vue'
import Field from '@/components/ui/Field.vue'
import Dialog from '@/components/ui/Dialog.vue'
import ConfirmDialog from '@/components/ui/ConfirmDialog.vue'
import EmptyState from '@/components/ui/EmptyState.vue'
import Spinner from '@/components/ui/Spinner.vue'
import Pagination from '@/components/ui/Pagination.vue'
import Switch from '@/components/ui/Switch.vue'
import ActionMenu from '@/components/ui/ActionMenu.vue'

const { t } = useI18n()
const router = useRouter()
const authStore = useAuthStore()
const toast = useToastStore()

const loading = ref(false)
const loadError = ref('')
const players = ref<Player[]>([])
const searchText = ref('')
const enabledFilter = ref<'all' | '0' | '1'>('all')
const onlineFilter = ref<'all' | '0' | '1'>('all')
const webAccessFilter = ref<'all' | '0' | '1'>('all')
const serverQuery = ref(false)
let loadSeq = 0
let searchTimer: ReturnType<typeof setTimeout> | undefined
const pagination = reactive({ currentPage: 1, pageSize: 20, total: 0 })
let persistReady = false

const displayedPlayers = computed(() => {
  if (serverQuery.value) return players.value
  return players.value.filter((p) => {
    if (enabledFilter.value !== 'all' && (p.enabled ? '1' : '0') !== enabledFilter.value) return false
    if (onlineFilter.value !== 'all' && (p.online ? '1' : '0') !== onlineFilter.value) return false
    if (webAccessFilter.value !== 'all' && (p.web_access ? '1' : '0') !== webAccessFilter.value) return false
    const q = searchText.value.trim().toLowerCase()
    if (!q) return true
    return p.username.toLowerCase().includes(q) || String(p.id).includes(q)
  })
})

const enabledOptions = computed(() => [
  { label: t('common.all'), value: 'all' },
  { label: t('player.enabledOnly'), value: '1' },
  { label: t('player.disabledOnly'), value: '0' },
])
const onlineOptions = computed(() => [
  { label: t('common.all'), value: 'all' },
  { label: t('player.onlineOnly'), value: '1' },
  { label: t('player.offlineOnly'), value: '0' },
])
const webAccessOptions = computed(() => [
  { label: t('common.all'), value: 'all' },
  { label: t('player.webAccessOn'), value: '1' },
  { label: t('player.webAccessOff'), value: '0' },
])

function restoreFilters() {
  const saved = loadFilters<{
    search?: string
    enabled?: typeof enabledFilter.value
    online?: typeof onlineFilter.value
    webAccess?: typeof webAccessFilter.value
    page?: number
    pageSize?: number
  }>(
    'players',
    accountScope(authStore.role, authStore.currentUserId),
  )
  if (!saved) return 1
  if (saved.search != null) searchText.value = saved.search
  if (saved.enabled) enabledFilter.value = saved.enabled
  if (saved.online) onlineFilter.value = saved.online
  if (saved.webAccess) webAccessFilter.value = saved.webAccess
  if (saved.pageSize) pagination.pageSize = saved.pageSize
  return saved.page || 1
}

watch([searchText, enabledFilter, onlineFilter, webAccessFilter, () => pagination.currentPage, () => pagination.pageSize], () => {
  if (!persistReady) return
  saveFilters('players', accountScope(authStore.role, authStore.currentUserId), {
    search: searchText.value,
    enabled: enabledFilter.value,
    online: onlineFilter.value,
    webAccess: webAccessFilter.value,
    page: pagination.currentPage,
    pageSize: pagination.pageSize,
  })
})

function buildPlayerRequest(page: number): PlayerListRequest {
  const req: PlayerListRequest = { page_number: page - 1, page_size: pagination.pageSize }
  const q = clampQuery(searchText.value)
  if (q) req.q = q
  if (enabledFilter.value !== 'all') req.enabled = Number(enabledFilter.value)
  if (onlineFilter.value !== 'all') req.online = Number(onlineFilter.value)
  if (webAccessFilter.value !== 'all') req.web_access = Number(webAccessFilter.value)
  return req
}

async function loadData(page = 1) {
  const seq = ++loadSeq
  loading.value = true
  loadError.value = ''
  pagination.currentPage = page
  try {
    const res = await playerApi.list(buildPlayerRequest(page))
    if (seq !== loadSeq) return
    if (isApiErrorPayload(res.data)) {
      loadError.value = messageFromResult(res.data as ApiResultLike, t, t('common.loadFailed'))
      players.value = []
      pagination.total = 0
      serverQuery.value = false
      return
    }
    serverQuery.value = supportsListQuery(res.data)
    players.value = res.data.players ?? []
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

watch([enabledFilter, onlineFilter, webAccessFilter], () => {
  if (persistReady) loadData(1)
})

onBeforeUnmount(() => {
  if (searchTimer) clearTimeout(searchTimer)
})

const addOpen = ref(false)
const addLoading = ref(false)
const addForm = reactive({ username: '', password: '' })
const addErrors = reactive({ username: '', password: '' })
const addDirty = computed(() => addOpen.value && (!!addForm.username || !!addForm.password))
const { confirmDiscard } = useUnsavedGuard(addDirty, () => t('common.unsavedLeave'))

function openAdd() {
  addForm.username = ''
  addForm.password = ''
  addErrors.username = ''
  addErrors.password = ''
  addOpen.value = true
}

function onAddOpen(next: boolean) {
  if (!next && !confirmDiscard()) return
  addOpen.value = next
}

function validateAdd() {
  addErrors.username = !addForm.username.trim()
    ? t('player.validation.usernameRequired')
    : (addForm.username.length > 30 ? t('player.validation.username') : '')
  addErrors.password = !addForm.password
    ? t('player.validation.passwordRequired')
    : (addForm.password.length > 15 ? t('player.validation.password') : '')
  return !addErrors.username && !addErrors.password
}

async function handleAdd() {
  if (addLoading.value || !validateAdd()) return
  addLoading.value = true
  try {
    const res = await playerApi.add({ ...addForm })
    if (res.data.code === 0) {
      toast.success(t('player.addSuccess'))
      addOpen.value = false
      loadData(1)
    } else {
      toast.error(messageFromResult(res.data, t, t('common.failed')))
    }
  } catch (err) {
    toast.error(messageFromUnknown(err, t, t('common.failed')))
  } finally {
    addLoading.value = false
  }
}

type PlayerAction = 'status' | 'kick'
const action = reactive({
  open: false,
  loading: false,
  kind: '' as PlayerAction | '',
  target: null as Player | null,
  title: '',
  message: '',
  confirmText: '',
  tone: 'warning' as 'warning' | 'success' | 'danger',
  details: [] as { label: string; value: string | number }[],
})

function detailsOf(player: Player) {
  return [
    { label: t('common.id'), value: player.id },
    { label: t('player.username'), value: player.username },
    { label: t('player.table.status'), value: player.online ? t('common.online') : t('common.offline') },
  ]
}

function handleToggleStatus(player: Player) {
  if (!authStore.isAdmin) return
  action.kind = 'status'
  action.target = player
  action.title = t(player.enabled ? 'player.disableTitle' : 'player.enableTitle')
  action.message = t(player.enabled ? 'player.disableConfirm' : 'player.enableConfirm', { name: player.username })
  action.confirmText = t(player.enabled ? 'common.disable' : 'common.enable')
  action.tone = player.enabled ? 'warning' : 'success'
  action.details = detailsOf(player)
  action.loading = false
  action.open = true
}

function handleKick(player: Player) {
  if (!authStore.isAdmin) return
  action.kind = 'kick'
  action.target = player
  action.title = t('player.kickTitle')
  action.message = t('player.kickConfirm', { name: player.username })
  action.confirmText = t('common.confirm')
  action.tone = 'warning'
  action.details = detailsOf(player)
  action.loading = false
  action.open = true
}

async function handleActionConfirm() {
  const target = action.target
  if (!target || !action.kind || action.loading) return
  action.loading = true
  try {
    if (action.kind === 'status') {
      const wasEnabled = target.enabled
      const res = await playerApi.updateStatus({ id: target.id, enabled: wasEnabled ? 0 : 1 })
      if (res.data.code === 0) {
        toast.success(wasEnabled ? t('player.disableSuccess') : t('player.enableSuccess'))
        action.open = false
        loadData(pagination.currentPage)
      } else {
        toast.error(messageFromResult(res.data, t, t('common.failed')))
      }
      return
    }
    const res = await playerApi.kick({ id: target.id })
    if (res.data.code === 0) {
      toast.success(t('player.kickSuccess'))
      action.open = false
      loadData(pagination.currentPage)
    } else {
      toast.error(messageFromResult(res.data, t, t('common.failed')))
    }
  } catch (err) {
    toast.error(messageFromUnknown(err, t, t('common.failed')))
  } finally {
    action.loading = false
  }
}

function goDetail(player: Player) {
  router.push({ name: 'PlayerDetail', params: { id: player.id } })
}

onMounted(async () => {
  const page = restoreFilters()
  persistReady = true
  await loadData(page)
})
</script>

<template>
  <div>
    <PageHeader :title="t('player.title')" :subtitle="t('player.subtitle')">
      <Button icon="refresh" :loading="loading" @click="loadData(pagination.currentPage)">{{ t('common.refresh') }}</Button>
      <Button v-if="authStore.isAdmin" variant="primary" icon="plus" @click="openAdd">{{ t('player.add') }}</Button>
    </PageHeader>

    <div class="ui-card overflow-hidden">
      <div class="flex flex-wrap items-center justify-between gap-3 border-b border-line px-4 py-3">
        <div class="flex min-w-0 flex-1 flex-wrap items-center gap-2">
          <div class="w-32">
            <Select :model-value="enabledFilter" @update:model-value="enabledFilter = String($event) as typeof enabledFilter">
              <option v-for="opt in enabledOptions" :key="opt.value" :value="opt.value">{{ opt.label }}</option>
            </Select>
          </div>
          <div class="w-32">
            <Select :model-value="onlineFilter" @update:model-value="onlineFilter = String($event) as typeof onlineFilter">
              <option v-for="opt in onlineOptions" :key="opt.value" :value="opt.value">{{ opt.label }}</option>
            </Select>
          </div>
          <div class="w-36">
            <Select :model-value="webAccessFilter" @update:model-value="webAccessFilter = String($event) as typeof webAccessFilter">
              <option v-for="opt in webAccessOptions" :key="opt.value" :value="opt.value">{{ opt.label }}</option>
            </Select>
          </div>
        </div>
        <div class="w-full max-w-xs">
          <Input :model-value="searchText" :placeholder="t('player.searchPlaceholder')" @update:model-value="searchText = String($event); scheduleSearch()" />
        </div>
      </div>
      <p class="border-b border-line px-4 py-2 text-xs text-faint">{{ serverQuery ? t('common.serverFilterActive') : t('common.currentPageFilterHint') }}</p>

      <Spinner v-if="loading && !players.length" :label="t('common.loading')" />
      <EmptyState v-else-if="loadError" :title="t('common.loadFailed')" :description="loadError" icon="warning" retry @retry="loadData(pagination.currentPage)" />
      <EmptyState v-else-if="!displayedPlayers.length" :title="searchText ? t('common.noResults') : t('player.empty')" icon="user" />

      <div v-else>
        <div class="divide-y divide-line md:hidden">
          <article v-for="player in displayedPlayers" :key="player.id" class="flex items-center justify-between gap-3 px-4 py-4">
            <button type="button" class="min-w-0 text-left" @click="goDetail(player)">
              <p class="truncate font-medium">{{ player.username }}</p>
              <p class="mt-1"><StatusBadge :pulse="player.online" :tone="player.online ? 'success' : 'neutral'" :label="player.online ? t('common.online') : t('common.offline')" /></p>
            </button>
            <div class="flex shrink-0 items-center gap-2">
              <Switch v-if="authStore.isAdmin" :model-value="player.enabled" @update:model-value="handleToggleStatus(player)" />
              <Button size="sm" variant="ghost" @click="goDetail(player)">{{ t('player.detail') }}</Button>
              <ActionMenu
                v-if="authStore.isAdmin && player.online"
                :items="[{ key: 'kick', label: t('player.kick'), icon: 'kick', tone: 'danger' }]"
                @select="handleKick(player)"
              />
            </div>
          </article>
        </div>

        <div class="hidden overflow-x-auto md:block">
          <table class="min-w-full text-left text-sm">
            <thead class="border-b border-line bg-canvas text-xs uppercase tracking-wide text-faint">
              <tr>
                <th class="px-4 py-3 font-medium">{{ t('player.table.username') }}</th>
                <th class="px-4 py-3 font-medium">{{ t('player.table.status') }}</th>
                <th class="px-4 py-3 font-medium">{{ t('player.table.account') }}</th>
                <th class="px-4 py-3 font-medium">{{ t('common.actions') }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="player in displayedPlayers" :key="player.id" class="border-b border-line last:border-0 hover:bg-canvas/80">
                <td class="px-4 py-3.5">
                  <button type="button" class="text-left font-medium hover:text-accent" @click="goDetail(player)">{{ player.username }}</button>
                  <p class="text-xs text-faint">#{{ player.id }}</p>
                </td>
                <td class="px-4 py-3.5">
                  <StatusBadge :pulse="player.online" :tone="player.online ? 'success' : 'neutral'" :label="player.online ? t('common.online') : t('common.offline')" />
                </td>
                <td class="px-4 py-3.5">
                  <Switch v-if="authStore.isAdmin" :model-value="player.enabled" @update:model-value="handleToggleStatus(player)" />
                  <StatusBadge v-else :tone="player.enabled ? 'success' : 'danger'" :label="player.enabled ? t('player.enabled') : t('player.disabled')" />
                </td>
                <td class="px-4 py-3.5">
                  <div class="flex items-center gap-1">
                    <Button size="sm" variant="ghost" @click="goDetail(player)">{{ t('player.detail') }}</Button>
                    <ActionMenu
                      v-if="authStore.isAdmin && player.online"
                      :items="[{ key: 'kick', label: t('player.kick'), icon: 'kick', tone: 'danger' }]"
                      @select="handleKick(player)"
                    />
                  </div>
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

    <Dialog :open="addOpen" :title="t('player.addTitle')" @update:open="onAddOpen">
      <div class="space-y-3">
        <Field :label="t('player.username')" required :error="addErrors.username">
          <Input v-model="addForm.username" :invalid="!!addErrors.username" autocomplete="off" />
        </Field>
        <Field :label="t('player.password')" required :error="addErrors.password">
          <Input v-model="addForm.password" type="password" :invalid="!!addErrors.password" autocomplete="new-password" />
        </Field>
      </div>
      <template #footer>
        <Button variant="secondary" :disabled="addLoading" @click="onAddOpen(false)">{{ t('common.cancel') }}</Button>
        <Button variant="primary" :loading="addLoading" @click="handleAdd">{{ t('common.confirm') }}</Button>
      </template>
    </Dialog>

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
  </div>
</template>
