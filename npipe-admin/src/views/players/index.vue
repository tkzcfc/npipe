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
          <el-button type="primary" :icon="Plus" @click="openAddDialog">{{ $t('player.add') }}</el-button>
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
            <el-tag :type="row.online ? 'success' : 'info'" size="small">
              {{ row.online ? $t('common.online') : $t('common.offline') }}
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
                  <el-dropdown-item @click="openRenameDialog(row)">
                    <el-icon><Edit /></el-icon> {{ $t('player.rename') }}
                  </el-dropdown-item>
                  <el-dropdown-item @click="openPasswordDialog(row)">
                    <el-icon><Lock /></el-icon> {{ $t('player.resetPassword') }}
                  </el-dropdown-item>
                  <el-dropdown-item :disabled="!row.online" @click="handleKick(row)">
                    <el-icon><SwitchButton /></el-icon> {{ $t('player.kick') }}
                  </el-dropdown-item>
                  <el-dropdown-item divided @click="handleRemove(row)">
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
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Plus, Refresh, Search, Edit, Delete, SwitchButton, MoreFilled, Lock } from '@element-plus/icons-vue'
import { playerApi } from '@/api'
import type { Player } from '@/types'

const { t } = useI18n()

// ── State ───────────────────────────────────────────────────────────────────
const loading = ref(false)
const players = ref<Player[]>([])
const searchText = ref('')

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
</style>

