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
          <el-button :icon="Refresh" :loading="loading" @click="loadData(pagination.currentPage)">{{ $t('common.refresh') }}</el-button>
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
        <template #empty>
          <el-empty :description="loadError || '暂无用户数据'">
            <el-button v-if="loadError" type="primary" :icon="Refresh" @click="loadData(pagination.currentPage)">
              重试
            </el-button>
          </el-empty>
        </template>
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
        <el-table-column :label="$t('player.table.actions')" width="240" fixed="right">
          <template #default="{ row }">
            <div class="row-actions">
              <el-button size="small" type="primary" text :icon="View" @click="openDetailDialog(row)">
                {{ $t('player.detail') }}
              </el-button>
              <template v-if="authStore.isAdmin">
                <el-button
                  size="small"
                  text
                  :type="row.enabled ? 'warning' : 'success'"
                  :icon="row.enabled ? CircleClose : SuccessFilled"
                  @click="handleToggleStatus(row)"
                >
                  {{ row.enabled ? $t('player.disable') : $t('player.enable') }}
                </el-button>
                <el-button
                  size="small"
                  text
                  :icon="SwitchButton"
                  :disabled="!row.online"
                  @click="handleKick(row)"
                >
                  {{ $t('player.kick') }}
                </el-button>
              </template>
            </div>
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

    <ConfirmAction
      v-model:visible="actionDialog.visible"
      :title="actionDialog.title"
      :message="actionDialog.message"
      :details="actionDialog.details"
      :loading="actionDialog.loading"
      :confirm-text="actionDialog.confirmText"
      :cancel-text="$t('common.cancel')"
      :confirm-type="actionDialog.confirmType"
      @confirm="handleActionConfirm"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { ElMessage, type FormInstance, type FormRules } from 'element-plus'
import { CircleClose, Plus, Refresh, Search, SuccessFilled, SwitchButton, View } from '@element-plus/icons-vue'
import { playerApi } from '@/api'
import { useAuthStore } from '@/stores/auth'
import ConfirmAction from '@/components/ConfirmAction.vue'
import type { Player } from '@/types'

const { t } = useI18n()
const router = useRouter()
const authStore = useAuthStore()

// ── State ───────────────────────────────────────────────────────────────────
const loading = ref(false)
const loadError = ref('')
const players = ref<Player[]>([])
const searchText = ref('')
let loadSeq = 0

const pagination = reactive({
  currentPage: 1,
  pageSize: 20,
  total: 0,
})

// ── Computed ─────────────────────────────────────────────────────────────────
const displayedPlayers = computed(() => {
  if (!searchText.value) return players.value
  const q = searchText.value.toLowerCase()
  return players.value.filter(p => p.username.toLowerCase().includes(q))
})

// ── Data ─────────────────────────────────────────────────────────────────────
async function loadData(page = 1) {
  const seq = ++loadSeq
  loading.value = true
  loadError.value = ''
  pagination.currentPage = page
  try {
    const res = await playerApi.list({ page_number: page - 1, page_size: pagination.pageSize })
    if (seq !== loadSeq) return
    players.value          = res.data.players ?? []
    pagination.total       = res.data.total_count ?? 0
  } catch {
    if (seq !== loadSeq) return
    loadError.value = '数据加载失败，请稍后刷新重试'
  } finally {
    if (seq === loadSeq) loading.value = false
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

type PlayerAction = 'status' | 'kick'

const actionDialog = reactive({
  visible: false,
  loading: false,
  action: '' as PlayerAction | '',
  target: null as Player | null,
  title: '',
  message: '',
  confirmText: '',
  confirmType: 'warning' as 'primary' | 'success' | 'warning' | 'danger',
  details: [] as { label: string; value: string | number }[],
})

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

function openDetailDialog(player: Player) {
  router.push({ name: 'PlayerDetail', params: { id: player.id } })
}

function handleToggleStatus(player: Player) {
  if (!authStore.isAdmin) return
  actionDialog.action = 'status'
  actionDialog.target = player
  actionDialog.title = t(player.enabled ? 'player.disableTitle' : 'player.enableTitle')
  actionDialog.message = t(player.enabled ? 'player.disableConfirm' : 'player.enableConfirm', { name: player.username })
  actionDialog.confirmText = t(player.enabled ? 'common.disable' : 'common.enable')
  actionDialog.confirmType = player.enabled ? 'warning' : 'success'
  actionDialog.details = playerDetails(player)
  actionDialog.loading = false
  actionDialog.visible = true
}

function handleKick(player: Player) {
  if (!authStore.isAdmin) return
  actionDialog.action = 'kick'
  actionDialog.target = player
  actionDialog.title = t('player.kickTitle')
  actionDialog.message = t('player.kickConfirm', { name: player.username })
  actionDialog.confirmText = t('common.confirm')
  actionDialog.confirmType = 'warning'
  actionDialog.details = playerDetails(player)
  actionDialog.loading = false
  actionDialog.visible = true
}

async function handleActionConfirm() {
  const target = actionDialog.target
  if (!target || !actionDialog.action) return

  actionDialog.loading = true
  try {
    if (actionDialog.action === 'status') {
      const wasEnabled = target.enabled
      const res = await playerApi.updateStatus({ id: target.id, enabled: wasEnabled ? 0 : 1 })
      if (res.data.code === 0) {
        ElMessage.success(wasEnabled ? t('player.disableSuccess') : t('player.enableSuccess'))
        actionDialog.visible = false
        loadData(pagination.currentPage)
      } else {
        ElMessage.error(res.data.msg || t('common.failed'))
      }
      return
    }

    const res = await playerApi.kick({ id: target.id })
    if (res.data.code === 0) {
      ElMessage.success(t('player.kickSuccess'))
      actionDialog.visible = false
      loadData(pagination.currentPage)
    } else {
      ElMessage.error(res.data.msg || t('common.failed'))
    }
  } finally {
    actionDialog.loading = false
  }
}

function playerDetails(player: Player) {
  return [
    { label: t('common.id'), value: player.id },
    { label: t('player.username'), value: player.username },
    { label: t('player.table.status'), value: player.online ? t('common.online') : t('common.offline') },
  ]
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

.row-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 2px;
  white-space: nowrap;

  :deep(.el-button + .el-button) {
    margin-left: 0;
  }
}

@media (max-width: 720px) {
  .table-toolbar {
    align-items: stretch;
  }
}
</style>
