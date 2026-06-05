<template>
  <div class="page-container">
    <div class="page-head">
      <div>
        <h1>{{ $t('tunnel.title') }}</h1>
        <p>{{ $t('tunnel.subtitle') }}</p>
      </div>
    </div>

    <section class="panel">
      <!-- Toolbar -->
      <div class="table-toolbar">
        <div style="display: flex; gap: 8px; flex-wrap: wrap;">
          <el-button type="primary" :icon="Plus" @click="openAddDialog">{{ $t('tunnel.add') }}</el-button>
          <el-button :icon="Refresh" :loading="loading" @click="loadData(pagination.currentPage)">{{ $t('common.refresh') }}</el-button>
          <el-button text @click="clearSearch">{{ $t('common.all') }}</el-button>
        </div>
        <el-input
          v-model="searchText"
          :placeholder="$t('tunnel.searchPlaceholder')"
          clearable
          style="width: 240px;"
          :prefix-icon="Search"
          @input="onSearch"
          @clear="clearSearch"
        />
      </div>

      <!-- Table -->
      <el-table
        v-loading="loading"
        :data="displayedTunnels"
        stripe
        row-key="id"
        style="width: 100%; margin-top: 16px;"
        :default-sort="{ prop: 'id', order: 'ascending' }"
      >
        <template #empty>
          <el-empty :description="loadError || '暂无隧道数据'">
            <el-button v-if="loadError" type="primary" :icon="Refresh" @click="loadData(pagination.currentPage)">
              重试
            </el-button>
          </el-empty>
        </template>
        <el-table-column prop="id" :label="$t('tunnel.table.id')" width="70" sortable />

        <el-table-column :label="$t('tunnel.table.source')" min-width="150">
          <template #default="{ row }">
            <code class="addr-code">{{ row.source }}</code>
          </template>
        </el-table-column>

        <el-table-column :label="$t('tunnel.table.endpoint')" min-width="150">
          <template #default="{ row }">
            <code v-if="row.endpoint" class="addr-code">{{ row.endpoint }}</code>
            <span v-else class="text-muted">—</span>
          </template>
        </el-table-column>

        <el-table-column :label="$t('tunnel.table.type')" width="90">
          <template #default="{ row }">
            <el-tag :type="tunnelTypeColor(row.tunnel_type)" size="small">
              {{ TUNNEL_TYPE_NAMES[row.tunnel_type] ?? row.tunnel_type }}
            </el-tag>
          </template>
        </el-table-column>

        <el-table-column :label="$t('tunnel.table.sender')" width="80">
          <template #default="{ row }">
            <span v-if="row.sender === 0" class="text-muted" style="font-size:12px;">{{ $t('common.server') }}</span>
            <span v-else class="font-mono">{{ row.sender }}</span>
          </template>
        </el-table-column>

        <el-table-column :label="$t('tunnel.table.receiver')" width="80">
          <template #default="{ row }">
            <span v-if="row.receiver === 0" class="text-muted" style="font-size:12px;">{{ $t('common.server') }}</span>
            <span v-else class="font-mono">{{ row.receiver }}</span>
          </template>
        </el-table-column>

        <el-table-column :label="$t('tunnel.table.runtime')" width="120">
          <template #default="{ row }">
            <el-tooltip
              :content="runtimeTip(row)"
              placement="top"
            >
              <el-tag :type="runtimeTagType(row)" size="small">
                {{ runtimeLabel(row) }}
              </el-tag>
            </el-tooltip>
          </template>
        </el-table-column>

        <el-table-column :label="$t('tunnel.table.encryption')" width="110">
          <template #default="{ row }">
            <el-tag
              v-if="row.encryption_method && row.encryption_method !== 'None'"
              type="primary" size="small"
            >
              {{ row.encryption_method }}
            </el-tag>
            <span v-else class="text-muted" style="font-size:12px;">{{ $t('common.none') }}</span>
          </template>
        </el-table-column>

        <el-table-column :label="$t('tunnel.table.compression')" width="75">
          <template #default="{ row }">
            <el-icon v-if="row.is_compressed" color="#3fb950"><SuccessFilled /></el-icon>
            <el-icon v-else style="color: var(--border-color)"><CircleClose /></el-icon>
          </template>
        </el-table-column>

        <el-table-column :label="$t('tunnel.table.status')" width="90">
          <template #default="{ row }">
            <el-tag :type="row.enabled ? 'success' : 'danger'" size="small">
              {{ row.enabled ? $t('common.enable') : $t('common.disable') }}
            </el-tag>
          </template>
        </el-table-column>

        <el-table-column prop="description" :label="$t('tunnel.table.description')" min-width="120" show-overflow-tooltip />

        <el-table-column :label="$t('tunnel.table.actions')" width="100" fixed="right">
          <template #default="{ row }">
            <el-dropdown trigger="click">
              <el-button size="small" text type="primary" style="font-size:16px; padding:0 8px;" :loading="toggling.has(row.id)">
                <el-icon><MoreFilled /></el-icon>
              </el-button>
              <template #dropdown>
                <el-dropdown-menu>
                  <el-dropdown-item v-if="canManageTunnel(row)" @click="openEditDialog(row)">
                    <el-icon><Edit /></el-icon> {{ $t('tunnel.edit') }}
                  </el-dropdown-item>
                  <el-dropdown-item v-if="canManageTunnel(row)" @click="openCloneDialog(row)">
                    <el-icon><CopyDocument /></el-icon> {{ $t('tunnel.clone') }}
                  </el-dropdown-item>
                  <el-dropdown-item v-if="canManageTunnel(row)" @click="handleToggle(row)">
                    <el-icon><SwitchButton /></el-icon> {{ row.enabled ? $t('common.disable') : $t('common.enable') }}
                  </el-dropdown-item>
                  <el-dropdown-item v-if="canManageTunnel(row)" divided @click="handleRemove(row)">
                    <el-icon><Delete /></el-icon> {{ $t('common.delete') }}
                  </el-dropdown-item>
                </el-dropdown-menu>
              </template>
            </el-dropdown>
          </template>
        </el-table-column>
      </el-table>

      <!-- Pagination -->
      <div style="margin-top: 16px; display:flex; justify-content:flex-end;">
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

    <!-- Add / Edit Dialog -->
    <el-dialog
      v-model="formDialog.visible"
      :title="formDialog.isEdit ? $t('tunnel.editTitle') : $t('tunnel.addTitle')"
      width="520px"
      destroy-on-close
    >
      <el-form
        ref="tunnelFormRef"
        :model="formDialog.form"
        :rules="tunnelRules"
        label-width="100px"
        @submit.prevent
      >
        <el-form-item :label="$t('common.type')" prop="tunnel_type">
          <el-select v-model="formDialog.form.tunnel_type" style="width:100%;" @change="onTypeChange">
            <el-option label="TCP"    :value="0" />
            <el-option label="UDP"    :value="1" />
            <el-option label="SOCKS5" :value="2" />
            <el-option label="HTTP"   :value="3" />
          </el-select>
        </el-form-item>

        <el-form-item :label="$t('tunnel.source')" prop="source">
          <el-input v-model="formDialog.form.source" :placeholder="$t('tunnel.sourcePlaceholder')" />
        </el-form-item>

        <el-form-item v-if="!isProxyType" :label="$t('tunnel.endpoint')" prop="endpoint">
          <el-input v-model="formDialog.form.endpoint" :placeholder="$t('tunnel.endpointPlaceholder')" />
        </el-form-item>

        <el-form-item :label="$t('tunnel.senderId')">
          <el-input-number
            v-if="authStore.isAdmin"
            v-model="formDialog.form.sender"
            :min="0"
            style="width:100%;"
          />
          <el-select v-else v-model="formDialog.form.sender" style="width:100%;">
            <el-option :label="$t('tunnel.endpointOption.self')" :value="authStore.currentUserId" />
          </el-select>
          <div class="form-hint">{{ $t('tunnel.hintServer') }}</div>
        </el-form-item>

        <el-form-item :label="$t('tunnel.receiverId')">
          <el-input-number
            v-if="authStore.isAdmin"
            v-model="formDialog.form.receiver"
            :min="0"
            style="width:100%;"
          />
          <el-select v-else v-model="formDialog.form.receiver" style="width:100%;">
            <el-option :label="$t('tunnel.endpointOption.server')" :value="0" />
            <el-option :label="$t('tunnel.endpointOption.self')" :value="authStore.currentUserId" />
          </el-select>
          <div class="form-hint">{{ $t('tunnel.hintServer') }}</div>
        </el-form-item>

        <template v-if="isProxyType">
          <el-form-item :label="$t('tunnel.authUser')">
            <el-input v-model="formDialog.form.username" :placeholder="$t('common.optional')" />
          </el-form-item>
          <el-form-item :label="$t('tunnel.authPass')">
            <el-input v-model="formDialog.form.password" :placeholder="$t('common.optional')" />
          </el-form-item>
        </template>

        <el-form-item :label="$t('tunnel.encryption')">
          <el-select v-model="formDialog.form.encryption_method" style="width:100%;">
            <el-option label="None（不加密）"   value="None" />
            <el-option label="Xor（轻量混淆）"  value="Xor" />
            <el-option label="AES-128（强加密）" value="Aes128" />
          </el-select>
        </el-form-item>

        <el-form-item :label="$t('tunnel.compression')">
          <el-switch v-model="formDialog.form.is_compressed" />
        </el-form-item>

        <el-form-item v-if="formDialog.isEdit" :label="$t('tunnel.enabled')">
          <el-switch v-model="formDialog.form.enabled" />
        </el-form-item>

        <el-form-item :label="$t('common.description')">
          <el-input v-model="formDialog.form.description" :placeholder="$t('common.optional')" />
        </el-form-item>
      </el-form>

      <div v-if="diagnoseResult.items.length" class="diagnose-panel">
        <div class="diagnose-title">
          <span>{{ $t('tunnel.diagnoseResult') }}</span>
          <el-tag :type="diagnoseResult.ok ? 'success' : 'danger'" size="small">
            {{ diagnoseResult.ok ? $t('tunnel.diagnosePassed') : $t('tunnel.diagnoseFailed') }}
          </el-tag>
        </div>
        <div class="diagnose-list">
          <div v-for="item in diagnoseResult.items" :key="item.key" class="diagnose-item">
            <el-tag :type="diagnoseTagType(item.level)" size="small">{{ diagnoseLevelLabel(item.level) }}</el-tag>
            <span>{{ diagnoseMessage(item) }}</span>
          </div>
        </div>
      </div>

      <template #footer>
        <el-button @click="formDialog.visible = false">{{ $t('common.cancel') }}</el-button>
        <el-button :loading="diagnosing" @click="handleDiagnose">{{ $t('tunnel.diagnose') }}</el-button>
        <el-button type="primary" :loading="formDialog.loading" @click="handleSubmit">
          {{ formDialog.isEdit ? $t('common.save') : $t('common.add') }}
        </el-button>
      </template>
    </el-dialog>

    <ConfirmDelete
      v-model:visible="deleteDialog.visible"
      :title="$t('tunnel.deleteTitle')"
      :message="$t('tunnel.deleteConfirm', { desc: deleteDialog.source })"
      :details="deleteDialog.details"
      :loading="deleteDialog.loading"
      :confirm-text="$t('common.delete')"
      :cancel-text="$t('common.cancel')"
      @confirm="handleRemoveConfirm"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, type FormInstance, type FormRules } from 'element-plus'
import { Plus, Refresh, Search, Edit, Delete, MoreFilled, CopyDocument, SwitchButton } from '@element-plus/icons-vue'
import { tunnelApi } from '@/api'
import { useAuthStore } from '@/stores/auth'
import ConfirmDelete from '@/components/ConfirmDelete.vue'
import type { Tunnel, TunnelDetail, TunnelDiagnoseItem, TunnelDiagnoseResponse, TunnelMutateRequest } from '@/types'

const { t } = useI18n()
const authStore = useAuthStore()

// ── Constants ─────────────────────────────────────────────────────────────────
const TUNNEL_TYPE_NAMES: Record<number, string> = { 0: 'TCP', 1: 'UDP', 2: 'SOCKS5', 3: 'HTTP' }
type TagType = 'primary' | 'success' | 'warning' | 'info' | 'danger'
const TUNNEL_TYPE_COLORS: Record<number, TagType> = { 0: 'primary', 1: 'warning', 2: 'success', 3: 'info' }

function tunnelTypeColor(type: number): TagType | undefined {
  return TUNNEL_TYPE_COLORS[type]
}

function runtimeTagType(tunnel: Tunnel): TagType {
  if (!tunnel.enabled) return 'info'
  return tunnel.available ? 'success' : 'warning'
}

function runtimeLabel(tunnel: Tunnel): string {
  if (!tunnel.enabled) return t('tunnel.runtime.disabled')
  if (tunnel.available) return t('tunnel.runtime.available')
  return t('tunnel.runtime.waiting')
}

function endpointStatus(id: number, online: boolean): string {
  if (id === 0) return t('common.server')
  return online ? t('common.online') : t('common.offline')
}

function runtimeTip(tunnel: Tunnel): string {
  return `${t('tunnel.table.sender')}: ${endpointStatus(tunnel.sender, tunnel.sender_online)} / ${t('tunnel.table.receiver')}: ${endpointStatus(tunnel.receiver, tunnel.receiver_online)}`
}

function canManageTunnel(tunnel: Tunnel): boolean {
  return authStore.isAdmin ||
    (tunnel.sender === authStore.currentUserId &&
      (tunnel.receiver === 0 || tunnel.receiver === authStore.currentUserId))
}

// ── State ─────────────────────────────────────────────────────────────────────
const loading    = ref(false)
const loadError  = ref('')
const tunnels    = ref<Tunnel[]>([])
const searchText = ref('')
const toggling   = ref<Set<number>>(new Set())
const diagnosing = ref(false)
let loadSeq = 0
const diagnoseResult = reactive<TunnelDiagnoseResponse>({
  ok: false,
  items: [],
})

const deleteDialog = reactive({
  visible: false,
  loading: false,
  tunnelId: 0,
  source: '',
  details: [] as { label: string; value: string }[],
})

const pagination = reactive({
  currentPage: 1,
  pageSize: 20,
  total: 0,
})

// ── Computed ──────────────────────────────────────────────────────────────────
const displayedTunnels = computed(() => {
  if (!searchText.value) return tunnels.value
  const q = searchText.value.toLowerCase()
  return tunnels.value.filter(t =>
    t.source.toLowerCase().includes(q) ||
    t.endpoint.toLowerCase().includes(q) ||
    t.description.toLowerCase().includes(q)
  )
})

// ── Data ──────────────────────────────────────────────────────────────────────
async function loadData(page = 1) {
  const seq = ++loadSeq
  loading.value = true
  loadError.value = ''
  pagination.currentPage = page
  try {
    const res = await tunnelApi.list({ page_number: page - 1, page_size: pagination.pageSize })
    if (seq !== loadSeq) return
    tunnels.value    = res.data.tunnels ?? []
    pagination.total = res.data.total_count ?? 0
  } catch {
    if (seq !== loadSeq) return
    loadError.value = '数据加载失败，请稍后刷新重试'
  } finally {
    if (seq === loadSeq) loading.value = false
  }
}

function onSearch() { /* client-side filter */ }
function clearSearch() { searchText.value = '' }

// ── Form dialog ───────────────────────────────────────────────────────────────
interface TunnelForm {
  id: number
  source: string
  endpoint: string
  tunnel_type: number
  sender: number
  receiver: number
  username: string
  password: string
  encryption_method: string
  is_compressed: boolean
  enabled: boolean
  description: string
}

const defaultForm = (): TunnelForm => ({
  id: 0, source: '', endpoint: '', tunnel_type: 0,
  sender: 0, receiver: 0, username: '', password: '',
  encryption_method: 'Xor', is_compressed: true, enabled: true, description: '',
})

const tunnelFormRef  = ref<FormInstance>()
const formDialog = reactive<{
  visible: boolean; isEdit: boolean; loading: boolean; form: TunnelForm
}>({
  visible: false, isEdit: false, loading: false, form: defaultForm(),
})

const isProxyType = computed(() =>
  formDialog.form.tunnel_type === 2 || formDialog.form.tunnel_type === 3
)

const tunnelRules: FormRules = {
  source: [{ required: true, message: () => t('tunnel.validation.sourceRequired'), trigger: 'blur' }],
  endpoint: [
    {
      validator: (_rule, _val, cb) => {
        if (!isProxyType.value && !formDialog.form.endpoint) {
          cb(new Error(t('tunnel.validation.endpointRequired')))
        } else {
          cb()
        }
      },
      trigger: 'blur',
    },
  ],
}

function onTypeChange() {
  if (isProxyType.value) {
    formDialog.form.endpoint = ''
  } else {
    formDialog.form.username = ''
    formDialog.form.password = ''
  }
}

function openAddDialog() {
  formDialog.form   = defaultForm()
  if (!authStore.isAdmin) {
    formDialog.form.sender = authStore.currentUserId
    formDialog.form.receiver = 0
  }
  formDialog.isEdit = false
  clearDiagnoseResult()
  formDialog.visible = true
}

function formFromTunnel(tunnel: TunnelDetail): TunnelForm {
  return {
    id:                tunnel.id,
    source:            tunnel.source,
    endpoint:          tunnel.endpoint,
    tunnel_type:       tunnel.tunnel_type,
    sender:            tunnel.sender,
    receiver:          tunnel.receiver,
    username:          tunnel.username,
    password:          tunnel.password,
    encryption_method: tunnel.encryption_method || 'None',
    is_compressed:     tunnel.is_compressed,
    enabled:           tunnel.enabled,
    description:       tunnel.description,
  }
}

async function fetchTunnelDetail(id: number): Promise<TunnelDetail | null> {
  const res = await tunnelApi.detail({ id })
  const detail = res.data.tunnel
  if (!detail) {
    ElMessage.error(t('tunnel.notFound'))
    return null
  }
  return detail
}

async function openEditDialog(tunnel: Tunnel) {
  const detail = await fetchTunnelDetail(tunnel.id)
  if (!detail) return

  formDialog.form = formFromTunnel(detail)
  formDialog.isEdit  = true
  clearDiagnoseResult()
  formDialog.visible = true
}

async function openCloneDialog(tunnel: Tunnel) {
  const detail = await fetchTunnelDetail(tunnel.id)
  if (!detail) return

  formDialog.form = {
    id: 0,
    source: detail.source,
    endpoint: detail.endpoint,
    tunnel_type: detail.tunnel_type,
    sender: detail.sender,
    receiver: detail.receiver,
    username: detail.username,
    password: detail.password,
    encryption_method: detail.encryption_method || 'None',
    is_compressed: detail.is_compressed,
    enabled: true,
    description: detail.description ? `${detail.description} copy` : '',
  }
  formDialog.isEdit = false
  clearDiagnoseResult()
  formDialog.visible = true
}

function buildRequest(form: TunnelForm): TunnelMutateRequest {
  const sender = authStore.isAdmin ? form.sender : authStore.currentUserId
  const receiver = authStore.isAdmin ? form.receiver : form.receiver === authStore.currentUserId ? authStore.currentUserId : 0
  return {
    id:                form.id,
    source:            form.source,
    endpoint:          isProxyType.value ? '' : form.endpoint,
    enabled:           form.enabled ? 1 : 0,
    sender,
    receiver,
    description:       form.description,
    tunnel_type:       form.tunnel_type,
    password:          isProxyType.value ? form.password : '',
    username:          isProxyType.value ? form.username : '',
    is_compressed:     form.is_compressed ? 1 : 0,
    encryption_method: form.encryption_method,
    custom_mapping:    {},
  }
}

async function handleSubmit() {
  const valid = await tunnelFormRef.value?.validate().catch(() => false)
  if (!valid) return
  formDialog.loading = true
  try {
    const req = buildRequest(formDialog.form)
    const res = formDialog.isEdit
      ? await tunnelApi.update(req)
      : await tunnelApi.add({ ...req, enabled: 1 })
    if (res.data.code === 0) {
      ElMessage.success(formDialog.isEdit ? t('tunnel.saveSuccess') : t('tunnel.addSuccess'))
      formDialog.visible = false
      loadData(pagination.currentPage)
    } else {
      ElMessage.error(res.data.msg || t('common.failed'))
    }
  } finally {
    formDialog.loading = false
  }
}

function clearDiagnoseResult() {
  diagnoseResult.ok = false
  diagnoseResult.items = []
}

async function handleDiagnose() {
  const valid = await tunnelFormRef.value?.validate().catch(() => false)
  if (!valid) return

  diagnosing.value = true
  try {
    const res = await tunnelApi.diagnose({
      id: formDialog.isEdit ? formDialog.form.id : undefined,
      source: formDialog.form.source,
      endpoint: isProxyType.value ? '' : formDialog.form.endpoint,
      sender: authStore.isAdmin ? formDialog.form.sender : authStore.currentUserId,
      receiver: authStore.isAdmin ? formDialog.form.receiver : formDialog.form.receiver === authStore.currentUserId ? authStore.currentUserId : 0,
      tunnel_type: formDialog.form.tunnel_type,
    })
    diagnoseResult.ok = res.data.ok
    diagnoseResult.items = res.data.items ?? []
  } finally {
    diagnosing.value = false
  }
}

function diagnoseTagType(level: TunnelDiagnoseItem['level']): TagType {
  if (level === 'ok') return 'success'
  if (level === 'warn') return 'warning'
  return 'danger'
}

function diagnoseLevelLabel(level: TunnelDiagnoseItem['level']): string {
  return t(`tunnel.diagnoseLevel.${level}`)
}

function diagnoseMessage(item: TunnelDiagnoseItem): string {
  return t(`tunnel.diagnoseMessage.${item.key}.${item.level}`)
}

// ── Toggle ────────────────────────────────────────────────────────────────────
async function handleToggle(tunnel: Tunnel) {
  const s = new Set(toggling.value)
  s.add(tunnel.id)
  toggling.value = s

  try {
    const res = await tunnelApi.updateStatus({
      id: tunnel.id,
      enabled: tunnel.enabled ? 0 : 1,
    })
    if (res.data.code === 0) {
      ElMessage.success(t('tunnel.toggleSuccess'))
      loadData(pagination.currentPage)
    } else {
      ElMessage.error(res.data.msg || t('common.failed'))
    }
  } finally {
    const s2 = new Set(toggling.value)
    s2.delete(tunnel.id)
    toggling.value = s2
  }
}

// ── Remove ────────────────────────────────────────────────────────────────────
function handleRemove(tunnel: Tunnel) {
  deleteDialog.tunnelId = tunnel.id
  deleteDialog.source = tunnel.source
  deleteDialog.details = [
    { label: t('common.id'), value: String(tunnel.id) },
    { label: t('tunnel.source'), value: tunnel.source },
    { label: t('tunnel.endpoint'), value: tunnel.endpoint || '-' },
    { label: t('common.type'), value: TUNNEL_TYPE_NAMES[tunnel.tunnel_type] ?? String(tunnel.tunnel_type) },
  ]
  deleteDialog.loading = false
  deleteDialog.visible = true
}

async function handleRemoveConfirm() {
  deleteDialog.loading = true
  try {
    const res = await tunnelApi.remove({ id: deleteDialog.tunnelId })
    if (res.data.code === 0) {
      ElMessage.success(t('tunnel.deleteSuccess'))
      deleteDialog.visible = false
      loadData(pagination.currentPage)
    } else {
      ElMessage.error(res.data.msg || t('common.failed'))
    }
  } finally {
    deleteDialog.loading = false
  }
}

onMounted(() => loadData(1))
</script>

<style scoped lang="scss">
.table-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 12px;
}

.addr-code {
  background: var(--bg-primary);
  padding: 2px 6px;
  border-radius: 4px;
  font-family: 'JetBrains Mono', Consolas, monospace;
  font-size: 12px;
  color: var(--accent);
  border: 1px solid var(--border-color);
}

.form-hint {
  font-size: 12px;
  color: var(--text-muted);
  margin-top: 4px;
}

.diagnose-panel {
  border: 1px solid var(--border-color);
  border-radius: 6px;
  background: var(--bg-primary);
  padding: 12px;
  margin-top: 12px;
}

.diagnose-title {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
}

.diagnose-list {
  display: grid;
  gap: 8px;
  margin-top: 10px;
}

.diagnose-item {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  color: var(--text-secondary);
}
</style>

