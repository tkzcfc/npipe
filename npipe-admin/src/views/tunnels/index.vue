<template>
  <div class="page-container">
    <div class="flex-between" style="margin-bottom: 20px;">
      <div>
        <h2 style="margin: 0 0 2px; font-size: 20px; font-weight: 700;">{{ $t('tunnel.title') }}</h2>
        <span style="font-size: 13px; color: var(--text-muted);">{{ $t('tunnel.subtitle') }}</span>
      </div>
    </div>

    <el-card>
      <!-- Toolbar -->
      <div class="table-toolbar">
        <div style="display: flex; gap: 8px; flex-wrap: wrap;">
          <el-button type="primary" :icon="Plus" @click="openAddDialog">{{ $t('tunnel.add') }}</el-button>
          <el-button :icon="Refresh" @click="loadData(pagination.currentPage)">{{ $t('common.refresh') }}</el-button>
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

        <el-table-column :label="$t('tunnel.table.actions')" width="220" fixed="right">
          <template #default="{ row }">
            <el-button size="small" type="primary" text @click="openEditDialog(row)">
              <el-icon><Edit /></el-icon> {{ $t('tunnel.edit') }}
            </el-button>
            <el-button
              size="small"
              :type="row.enabled ? 'warning' : 'success'"
              text
              :loading="toggling.has(row.id)"
              @click="handleToggle(row)"
            >
              {{ row.enabled ? $t('common.disable') : $t('common.enable') }}
            </el-button>
            <el-button size="small" type="danger" text @click="handleRemove(row)">
              <el-icon><Delete /></el-icon> {{ $t('common.delete') }}
            </el-button>
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
    </el-card>

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
          <el-input-number v-model="formDialog.form.sender" :min="0" style="width:100%;" />
          <div class="form-hint">{{ $t('tunnel.hintServer') }}</div>
        </el-form-item>

        <el-form-item :label="$t('tunnel.receiverId')">
          <el-input-number v-model="formDialog.form.receiver" :min="0" style="width:100%;" />
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
      <template #footer>
        <el-button @click="formDialog.visible = false">{{ $t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="formDialog.loading" @click="handleSubmit">
          {{ formDialog.isEdit ? $t('common.save') : $t('common.add') }}
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Plus, Refresh, Search, Edit, Delete } from '@element-plus/icons-vue'
import { tunnelApi } from '@/api'
import type { Tunnel, TunnelMutateRequest } from '@/types'

const { t } = useI18n()

// ── Constants ─────────────────────────────────────────────────────────────────
const TUNNEL_TYPE_NAMES: Record<number, string> = { 0: 'TCP', 1: 'UDP', 2: 'SOCKS5', 3: 'HTTP' }
type TagType = 'primary' | 'success' | 'warning' | 'info' | 'danger'
const TUNNEL_TYPE_COLORS: Record<number, TagType> = { 0: 'primary', 1: 'warning', 2: 'success', 3: 'info' }

function tunnelTypeColor(type: number): TagType | undefined {
  return TUNNEL_TYPE_COLORS[type]
}

// ── State ─────────────────────────────────────────────────────────────────────
const loading    = ref(false)
const tunnels    = ref<Tunnel[]>([])
const searchText = ref('')
const toggling   = ref<Set<number>>(new Set())

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
  loading.value = true
  pagination.currentPage = page
  try {
    const res = await tunnelApi.list({ page_number: page - 1, page_size: pagination.pageSize })
    tunnels.value    = res.data.tunnels ?? []
    pagination.total = res.data.total_count ?? 0
  } finally {
    loading.value = false
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
  formDialog.isEdit = false
  formDialog.visible = true
}

function openEditDialog(tunnel: Tunnel) {
  formDialog.form = {
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
  formDialog.isEdit  = true
  formDialog.visible = true
}

function buildRequest(form: TunnelForm): TunnelMutateRequest {
  return {
    id:                form.id,
    source:            form.source,
    endpoint:          isProxyType.value ? '' : form.endpoint,
    enabled:           form.enabled ? 1 : 0,
    sender:            form.sender,
    receiver:          form.receiver,
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

// ── Toggle ────────────────────────────────────────────────────────────────────
async function handleToggle(tunnel: Tunnel) {
  const s = new Set(toggling.value)
  s.add(tunnel.id)
  toggling.value = s

  try {
    const req: TunnelMutateRequest = {
      id:                tunnel.id,
      source:            tunnel.source,
      endpoint:          tunnel.endpoint,
      enabled:           tunnel.enabled ? 0 : 1,
      sender:            tunnel.sender,
      receiver:          tunnel.receiver,
      description:       tunnel.description,
      tunnel_type:       tunnel.tunnel_type,
      password:          tunnel.password,
      username:          tunnel.username,
      is_compressed:     tunnel.is_compressed ? 1 : 0,
      encryption_method: tunnel.encryption_method,
      custom_mapping:    tunnel.custom_mapping ?? {},
    }
    const res = await tunnelApi.update(req)
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
async function handleRemove(tunnel: Tunnel) {
  await ElMessageBox.confirm(
    t('tunnel.deleteConfirm', { desc: tunnel.source }),
    t('tunnel.deleteTitle'), { type: 'warning', confirmButtonText: t('common.delete'), cancelButtonText: t('common.cancel') }
  )
  const res = await tunnelApi.remove({ id: tunnel.id })
  if (res.data.code === 0) {
    ElMessage.success(t('tunnel.deleteSuccess'))
    loadData(pagination.currentPage)
  } else {
    ElMessage.error(res.data.msg || t('common.failed'))
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
</style>

