<template>
  <div class="page-container">
    <div class="page-head">
      <div>
        <h1>{{ $t('player.title') }}</h1>
        <p>{{ $t('player.subtitle') }}</p>
      </div>
    </div>

    <section class="panel">
      <!-- Toolbar -->
      <div class="table-toolbar">
        <div style="display: flex; gap: 8px;">
          <el-button v-if="authStore.isAdmin" type="primary" :icon="Plus" @click="openAddDialog">{{ $t('player.add') }}</el-button>
          <el-button :icon="Refresh" @click="loadData(pagination.currentPage)">{{ $t('common.refresh') }}</el-button>
        </div>
        <el-input
          v-model="searchText"
          :placeholder="$t('player.searchPlaceholder')"
          clearable
          style="width: 220px;"
          :prefix-icon="Search"
          @input="onSearch"
          @clear="onSearch"
        />
      </div>

      <!-- Table -->
      <el-table
        v-loading="loading"
        :data="displayedPlayers"
        stripe
        row-key="id"
        style="width: 100%; margin-top: 16px;"
      >
        <el-table-column prop="id" :label="$t('player.table.id')" width="80" />
        <el-table-column prop="username" :label="$t('player.table.username')" min-width="160">
          <template #default="{ row }">
            <div style="display:flex; align-items:center; gap:8px;">
              <el-avatar :size="28" style="background: var(--accent); font-size: 13px; flex-shrink:0;">
                {{ row.username.charAt(0).toUpperCase() }}
              </el-avatar>
              <span class="font-mono">{{ row.username }}</span>
            </div>
          </template>
        </el-table-column>
        <el-table-column :label="$t('player.table.status')" width="110">
          <template #default="{ row }">
            <el-tag :type="row.enabled ? row.online ? 'success' : 'info' : 'danger'" size="small">
              {{ row.enabled ? row.online ? $t('common.online') : $t('common.offline') : $t('player.disabled') }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="$t('player.table.account')" width="100">
          <template #default="{ row }">
            <el-tag :type="row.enabled ? 'success' : 'danger'" size="small">
              {{ row.enabled ? $t('player.enabled') : $t('player.disabled') }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column v-if="authStore.isAdmin" :label="$t('player.table.webAccess')" width="110">
          <template #default="{ row }">
            <el-tag :type="row.web_access ? 'success' : 'info'" size="small">
              {{ row.web_access ? $t('player.allowed') : $t('player.notAllowed') }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="ip_addr" :label="$t('player.table.ip')" min-width="140">
          <template #default="{ row }">
            <span class="font-mono">{{ row.online ? row.ip_addr : '-' }}</span>
          </template>
        </el-table-column>
        <el-table-column :label="$t('player.table.onlineTime')" min-width="170">
          <template #default="{ row }">
            <span>{{ row.online && row.online_time ? formatTime(row.online_time) : '-' }}</span>
          </template>
        </el-table-column>
        <el-table-column label="↓ 入站" min-width="100" align="right">
          <template #default="{ row }">
            <span class="font-mono">{{ formatBytes(row.bytes_in) }}</span>
          </template>
        </el-table-column>
        <el-table-column label="↑ 出站" min-width="100" align="right">
          <template #default="{ row }">
            <span class="font-mono">{{ formatBytes(row.bytes_out) }}</span>
          </template>
        </el-table-column>
        <el-table-column :label="$t('player.table.actions')" width="100" fixed="right">
          <template #default="{ row }">
            <el-dropdown trigger="click">
              <el-button size="small" text type="primary" style="font-size:16px; padding:0 8px;">
                <el-icon><MoreFilled /></el-icon>
              </el-button>
              <template #dropdown>
                <el-dropdown-menu>
                  <el-dropdown-item @click="openDetailDialog(row)">
                    <el-icon><View /></el-icon> {{ $t('player.detail') }}
                  </el-dropdown-item>
                  <el-dropdown-item v-if="authStore.isAdmin" @click="openRenameDialog(row)">
                    <el-icon><Edit /></el-icon> {{ $t('player.rename') }}
                  </el-dropdown-item>
                  <el-dropdown-item @click="openPasswordDialog(row)">
                    <el-icon><Lock /></el-icon> {{ $t('player.resetPassword') }}
                  </el-dropdown-item>
                  <el-dropdown-item v-if="authStore.isAdmin" :disabled="!row.online" @click="handleKick(row)">
                    <el-icon><SwitchButton /></el-icon> {{ $t('player.kick') }}
                  </el-dropdown-item>
                  <el-dropdown-item v-if="authStore.isAdmin" @click="handleToggleStatus(row)">
                    <el-icon><CircleClose v-if="row.enabled" /><SuccessFilled v-else /></el-icon>
                    {{ row.enabled ? $t('player.disable') : $t('player.enable') }}
                  </el-dropdown-item>
                  <el-dropdown-item v-if="authStore.isAdmin" @click="handleToggleWebAccess(row)">
                    <el-icon><View /></el-icon>
                    {{ row.web_access ? $t('player.revokeWebAccess') : $t('player.grantWebAccess') }}
                  </el-dropdown-item>
                  <el-dropdown-item v-if="authStore.isAdmin" divided @click="handleRemove(row)">
                    <el-icon><Delete /></el-icon> {{ $t('player.delete') }}
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

    <!-- Add Dialog -->
    <el-dialog
      v-model="addDialog.visible"
      :title="$t('player.addTitle')"
      width="440px"
      destroy-on-close
    >
      <el-form
        ref="addFormRef"
        :model="addDialog.form"
        :rules="addRules"
        label-width="80px"
        @submit.prevent
      >
        <el-form-item :label="$t('player.username')" prop="username">
          <el-input v-model="addDialog.form.username" :placeholder="$t('login.usernamePlaceholder')" />
        </el-form-item>
        <el-form-item :label="$t('player.password')" prop="password">
          <el-input v-model="addDialog.form.password" type="password" show-password :placeholder="$t('login.passwordPlaceholder')" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="addDialog.visible = false">{{ $t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="addDialog.loading" @click="handleAdd">{{ $t('common.ok') }}</el-button>
      </template>
    </el-dialog>

    <!-- Rename Dialog -->
    <el-dialog
      v-model="renameDialog.visible"
      :title="$t('player.renameTitle')"
      width="440px"
      destroy-on-close
    >
      <el-form
        ref="renameFormRef"
        :model="renameDialog.form"
        :rules="renameRules"
        label-width="80px"
        @submit.prevent
      >
        <el-form-item :label="$t('common.id')">
          <el-input :value="renameDialog.form.id" readonly />
        </el-form-item>
        <el-form-item :label="$t('player.username')" prop="username">
          <el-input v-model="renameDialog.form.username" :placeholder="$t('login.usernamePlaceholder')" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="renameDialog.visible = false">{{ $t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="renameDialog.loading" @click="handleRename">{{ $t('common.save') }}</el-button>
      </template>
    </el-dialog>

    <!-- Reset Password Dialog -->
    <el-dialog
      v-model="passwordDialog.visible"
      :title="$t('player.resetPasswordTitle')"
      width="440px"
      destroy-on-close
    >
      <el-form
        ref="passwordFormRef"
        :model="passwordDialog.form"
        :rules="passwordRules"
        label-width="80px"
        @submit.prevent
      >
        <el-form-item :label="$t('common.id')">
          <el-input :value="passwordDialog.form.id" readonly />
        </el-form-item>
        <el-form-item :label="$t('player.newPassword')" prop="password">
          <el-input v-model="passwordDialog.form.password" type="password" show-password :placeholder="$t('login.passwordPlaceholder')" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="passwordDialog.visible = false">{{ $t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="passwordDialog.loading" @click="handleResetPassword">{{ $t('common.save') }}</el-button>
      </template>
    </el-dialog>

    <!-- Detail Dialog -->
    <el-dialog
      v-model="detailDialog.visible"
      :title="$t('player.detailTitle')"
      width="760px"
      destroy-on-close
    >
      <div v-loading="detailDialog.loading">
        <template v-if="detailDialog.data">
          <div class="detail-summary">
            <div class="detail-card">
              <span>{{ $t('player.table.status') }}</span>
              <strong>
                <el-tag :type="detailDialog.data.enabled ? detailDialog.data.online ? 'success' : 'info' : 'danger'" size="small">
                  {{ detailDialog.data.enabled ? detailDialog.data.online ? $t('common.online') : $t('common.offline') : $t('player.disabled') }}
                </el-tag>
              </strong>
            </div>
            <div class="detail-card">
              <span>{{ $t('player.detailTraffic24h') }}</span>
              <strong>{{ formatBytes(detailDialog.data.traffic_24h_in + detailDialog.data.traffic_24h_out) }}</strong>
            </div>
            <div class="detail-card">
              <span>{{ $t('player.detailTunnels') }}</span>
              <strong>{{ detailDialog.data.tunnels.length }}</strong>
            </div>
          </div>

          <el-descriptions :column="2" border size="small" style="margin-top: 14px;">
            <el-descriptions-item :label="$t('common.id')">{{ detailDialog.data.id }}</el-descriptions-item>
            <el-descriptions-item :label="$t('player.username')">{{ detailDialog.data.username }}</el-descriptions-item>
            <el-descriptions-item :label="$t('player.table.account')">
              {{ detailDialog.data.enabled ? $t('player.enabled') : $t('player.disabled') }}
            </el-descriptions-item>
            <el-descriptions-item :label="$t('player.table.ip')">{{ detailDialog.data.online ? detailDialog.data.ip_addr : '-' }}</el-descriptions-item>
            <el-descriptions-item :label="$t('player.table.onlineTime')">
              {{ detailDialog.data.online ? formatDuration(detailDialog.data.online_time) : '-' }}
            </el-descriptions-item>
            <el-descriptions-item :label="$t('player.createTime')">{{ detailDialog.data.create_time }}</el-descriptions-item>
            <el-descriptions-item :label="$t('player.currentTraffic')">
              ↓ {{ formatBytes(detailDialog.data.bytes_in) }} / ↑ {{ formatBytes(detailDialog.data.bytes_out) }}
            </el-descriptions-item>
          </el-descriptions>

          <h3 class="detail-title">{{ $t('player.associatedTunnels') }}</h3>
          <el-table :data="detailDialog.data.tunnels" size="small" max-height="220" empty-text="-">
            <el-table-column prop="id" label="ID" width="72" />
            <el-table-column :label="$t('tunnel.table.source')" min-width="150">
              <template #default="{ row }"><code class="font-mono">{{ row.source }}</code></template>
            </el-table-column>
            <el-table-column :label="$t('tunnel.table.type')" width="90">
              <template #default="{ row }">{{ TUNNEL_TYPE_NAMES[row.tunnel_type] ?? row.tunnel_type }}</template>
            </el-table-column>
            <el-table-column :label="$t('player.role')" width="90">
              <template #default="{ row }">{{ $t(`player.tunnelRole.${row.role}`) }}</template>
            </el-table-column>
            <el-table-column :label="$t('tunnel.table.runtime')" width="100">
              <template #default="{ row }">
                <el-tag :type="row.available ? 'success' : row.enabled ? 'warning' : 'info'" size="small">
                  {{ row.available ? $t('tunnel.runtime.available') : row.enabled ? $t('tunnel.runtime.waiting') : $t('tunnel.runtime.disabled') }}
                </el-tag>
              </template>
            </el-table-column>
          </el-table>

          <h3 class="detail-title">{{ $t('player.recentLogins') }}</h3>
          <el-table :data="detailDialog.data.recent_logins" size="small" max-height="220" empty-text="-">
            <el-table-column prop="ip_addr" :label="$t('player.table.ip')" min-width="140" />
            <el-table-column prop="login_time" :label="$t('loginLog.loginTime')" min-width="160" />
            <el-table-column :label="$t('loginLog.duration')" width="110">
              <template #default="{ row }">{{ row.logout_time ? formatDuration(row.duration_secs) : $t('loginLog.online') }}</template>
            </el-table-column>
          </el-table>
        </template>
      </div>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Plus, Refresh, Search, Edit, Delete, SwitchButton, MoreFilled, Lock, View, CircleClose, SuccessFilled } from '@element-plus/icons-vue'
import { playerApi } from '@/api'
import { useAuthStore } from '@/stores/auth'
import type { Player, PlayerDetail } from '@/types'

const { t } = useI18n()
const authStore = useAuthStore()

// ── State ───────────────────────────────────────────────────────────────────
const loading = ref(false)
const players = ref<Player[]>([])
const searchText = ref('')

const pagination = reactive({
  currentPage: 1,
  pageSize: 20,
  total: 0,
})
const TUNNEL_TYPE_NAMES: Record<number, string> = { 0: 'TCP', 1: 'UDP', 2: 'SOCKS5', 3: 'HTTP' }

// ── Computed ─────────────────────────────────────────────────────────────────
const displayedPlayers = computed(() => {
  if (!searchText.value) return players.value
  const q = searchText.value.toLowerCase()
  return players.value.filter(p => p.username.toLowerCase().includes(q))
})

// ── Data ─────────────────────────────────────────────────────────────────────
async function loadData(page = 1) {
  loading.value = true
  pagination.currentPage = page
  try {
    const res = await playerApi.list({ page_number: page - 1, page_size: pagination.pageSize })
    players.value          = res.data.players ?? []
    pagination.total       = res.data.total_count ?? 0
  } finally {
    loading.value = false
  }
}

function onSearch() {
  /* client-side filter, no extra request needed */
}

// ── Add ───────────────────────────────────────────────────────────────────────
const addFormRef  = ref<FormInstance>()
const addDialog = reactive({
  visible: false,
  loading: false,
  form: { username: '', password: '' },
})
const addRules: FormRules = {
  username: [{ required: true, message: () => t('player.validation.usernameRequired'), trigger: 'blur' },
             { min: 1, max: 30, message: () => t('player.validation.username'), trigger: 'blur' }],
  password: [{ required: true, message: () => t('player.validation.passwordRequired'), trigger: 'blur' },
             { min: 1, max: 15, message: () => t('player.validation.password'), trigger: 'blur' }],
}

function openAddDialog() {
  addDialog.form = { username: '', password: '' }
  addDialog.visible = true
}

async function handleAdd() {
  const valid = await addFormRef.value?.validate().catch(() => false)
  if (!valid) return
  addDialog.loading = true
  try {
    const res = await playerApi.add(addDialog.form)
    if (res.data.code === 0) {
      ElMessage.success(t('player.addSuccess'))
      addDialog.visible = false
      loadData(1)
    } else {
      ElMessage.error(res.data.msg || t('common.failed'))
    }
  } finally {
    addDialog.loading = false
  }
}

// ── Rename ─────────────────────────────────────────────────────────────────
const renameFormRef = ref<FormInstance>()
const renameDialog = reactive({
  visible: false,
  loading: false,
  form: { id: 0, username: '' },
})
const renameRules: FormRules = {
  username: [{ required: true, message: () => t('player.validation.usernameRequired'), trigger: 'blur' },
             { min: 1, max: 30, message: () => t('player.validation.username'), trigger: 'blur' }],
}

function openRenameDialog(player: Player) {
  if (!authStore.isAdmin) return
  renameDialog.form = { id: player.id, username: player.username }
  renameDialog.visible = true
}

async function handleRename() {
  const valid = await renameFormRef.value?.validate().catch(() => false)
  if (!valid) return
  renameDialog.loading = true
  try {
    const res = await playerApi.rename(renameDialog.form)
    if (res.data.code === 0) {
      ElMessage.success(t('player.saveSuccess'))
      renameDialog.visible = false
      loadData(pagination.currentPage)
    } else {
      ElMessage.error(res.data.msg || t('common.failed'))
    }
  } finally {
    renameDialog.loading = false
  }
}

// ── Reset password ───────────────────────────────────────────────────────────
const passwordFormRef = ref<FormInstance>()
const passwordDialog = reactive({
  visible: false,
  loading: false,
  form: { id: 0, password: '' },
})
const passwordRules: FormRules = {
  password: [{ required: true, message: () => t('player.validation.passwordRequired'), trigger: 'blur' },
             { min: 1, max: 15, message: () => t('player.validation.password'), trigger: 'blur' }],
}

function openPasswordDialog(player: Player) {
  passwordDialog.form = { id: player.id, password: '' }
  passwordDialog.visible = true
}

async function handleResetPassword() {
  const valid = await passwordFormRef.value?.validate().catch(() => false)
  if (!valid) return
  passwordDialog.loading = true
  try {
    const res = await playerApi.resetPassword(passwordDialog.form)
    if (res.data.code === 0) {
      ElMessage.success(t('player.passwordResetSuccess'))
      passwordDialog.visible = false
      loadData(pagination.currentPage)
    } else {
      ElMessage.error(res.data.msg || t('common.failed'))
    }
  } finally {
    passwordDialog.loading = false
  }
}

// ── Detail ───────────────────────────────────────────────────────────────────
const detailDialog = reactive<{
  visible: boolean
  loading: boolean
  data: PlayerDetail | null
}>({
  visible: false,
  loading: false,
  data: null,
})

async function openDetailDialog(player: Player) {
  detailDialog.visible = true
  detailDialog.loading = true
  detailDialog.data = null
  try {
    const res = await playerApi.detail({ id: player.id })
    if (res.data.player) {
      detailDialog.data = res.data.player
    } else {
      ElMessage.error(t('player.notFound'))
      detailDialog.visible = false
    }
  } finally {
    detailDialog.loading = false
  }
}

// ── Remove ────────────────────────────────────────────────────────────────────
async function handleRemove(player: Player) {
  await ElMessageBox.confirm(
    t('player.deleteConfirm', { name: player.username }),
    t('player.deleteTitle'), { type: 'warning', confirmButtonText: t('player.deleteBtn'), cancelButtonText: t('common.cancel') }
  )
  const res = await playerApi.remove({ id: player.id })
  if (res.data.code === 0) {
    ElMessage.success(t('player.deleteSuccess'))
    loadData(pagination.currentPage)
  } else {
    ElMessage.error(res.data.msg || t('common.failed'))
  }
}

// ── Kick ──────────────────────────────────────────────────────────────────────
async function handleKick(player: Player) {
  await ElMessageBox.confirm(
    t('player.kickConfirm', { name: player.username }),
    t('player.kickTitle'), { type: 'warning', confirmButtonText: t('common.confirm'), cancelButtonText: t('common.cancel') }
  )
  const res = await playerApi.kick({ id: player.id })
  if (res.data.code === 0) {
    ElMessage.success(t('player.kickSuccess'))
    loadData(pagination.currentPage)
  } else {
    ElMessage.error(res.data.msg || t('common.failed'))
  }
}

// ── Helpers ──────────────────────────────────────────────────────────────────
function formatTime(ts: number): string {
  const d = new Date(ts * 1000)
  const pad = (n: number) => n.toString().padStart(2, '0')
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
}

function formatBytes(n: number): string {
  if (n < 1024) return n + ' B'
  if (n < 1048576) return (n / 1024).toFixed(1) + ' KB'
  if (n < 1073741824) return (n / 1048576).toFixed(1) + ' MB'
  return (n / 1073741824).toFixed(2) + ' GB'
}

async function handleToggleStatus(player: Player) {
  await ElMessageBox.confirm(
    t(player.enabled ? 'player.disableConfirm' : 'player.enableConfirm', { name: player.username }),
    t(player.enabled ? 'player.disableTitle' : 'player.enableTitle'),
    { type: 'warning', confirmButtonText: t('common.confirm'), cancelButtonText: t('common.cancel') }
  )
  const res = await playerApi.updateStatus({ id: player.id, enabled: player.enabled ? 0 : 1 })
  if (res.data.code === 0) {
    ElMessage.success(player.enabled ? t('player.disableSuccess') : t('player.enableSuccess'))
    loadData(pagination.currentPage)
  } else {
    ElMessage.error(res.data.msg || t('common.failed'))
  }
}

async function handleToggleWebAccess(player: Player) {
  await ElMessageBox.confirm(
    t(player.web_access ? 'player.revokeWebAccessConfirm' : 'player.grantWebAccessConfirm', { name: player.username }),
    t(player.web_access ? 'player.revokeWebAccessTitle' : 'player.grantWebAccessTitle'),
    { type: 'warning', confirmButtonText: t('common.confirm'), cancelButtonText: t('common.cancel') }
  )
  const res = await playerApi.updateWebAccess({ id: player.id, web_access: player.web_access ? 0 : 1 })
  if (res.data.code === 0) {
    ElMessage.success(player.web_access ? t('player.revokeWebAccessSuccess') : t('player.grantWebAccessSuccess'))
    loadData(pagination.currentPage)
  } else {
    ElMessage.error(res.data.msg || t('common.failed'))
  }
}

function formatDuration(seconds: number): string {
  if (!seconds) return '0s'
  const h = Math.floor(seconds / 3600)
  const m = Math.floor((seconds % 3600) / 60)
  const s = seconds % 60
  if (h > 0) return `${h}h ${m}m`
  if (m > 0) return `${m}m ${s}s`
  return `${s}s`
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

.detail-summary {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 10px;
}

.detail-card {
  border: 1px solid var(--border-color);
  border-radius: 6px;
  padding: 10px 12px;
  background: var(--bg-primary);
  display: flex;
  flex-direction: column;
  gap: 6px;

  span {
    color: var(--text-secondary);
    font-size: 12px;
  }

  strong {
    color: var(--text-primary);
    font-size: 18px;
  }
}

.detail-title {
  margin: 18px 0 10px;
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
}

@media (max-width: 720px) {
  .detail-summary {
    grid-template-columns: 1fr;
  }
}
</style>

