<template>
  <div class="page-container">
    <div class="flex-between" style="margin-bottom: 20px;">
      <div>
        <h2 style="margin: 0 0 2px; font-size: 20px; font-weight: 700;">用户管理</h2>
        <span style="font-size: 13px; color: var(--text-muted);">管理所有注册用户</span>
      </div>
    </div>

    <el-card>
      <!-- Toolbar -->
      <div class="table-toolbar">
        <div style="display: flex; gap: 8px;">
          <el-button type="primary" :icon="Plus" @click="openAddDialog">添加用户</el-button>
          <el-button :icon="Refresh" @click="loadData(pagination.currentPage)">刷新</el-button>
        </div>
        <el-input
          v-model="searchText"
          placeholder="搜索用户名..."
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
        <el-table-column prop="id" label="ID" width="80" />
        <el-table-column prop="username" label="用户名" min-width="160">
          <template #default="{ row }">
            <div style="display:flex; align-items:center; gap:8px;">
              <el-avatar :size="28" style="background: var(--accent); font-size: 13px; flex-shrink:0;">
                {{ row.username.charAt(0).toUpperCase() }}
              </el-avatar>
              <span class="font-mono">{{ row.username }}</span>
            </div>
          </template>
        </el-table-column>
        <el-table-column prop="password" label="密码" min-width="140">
          <template #default="{ row }">
            <div style="display:flex; align-items:center; gap:6px;">
              <span class="font-mono">{{ showPasswords.has(row.id) ? row.password : '••••••••' }}</span>
              <el-button
                size="small" text
                :icon="showPasswords.has(row.id) ? Hide : View"
                @click="togglePassword(row.id)"
              />
            </div>
          </template>
        </el-table-column>
        <el-table-column label="在线状态" width="110">
          <template #default="{ row }">
            <el-tag :type="row.online ? 'success' : 'info'" size="small">
              {{ row.online ? '在线' : '离线' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="ip_addr" label="IP 地址" min-width="140">
          <template #default="{ row }">
            <span class="font-mono">{{ row.online ? row.ip_addr : '-' }}</span>
          </template>
        </el-table-column>
        <el-table-column label="上线时间" min-width="170">
          <template #default="{ row }">
            <span>{{ row.online && row.online_time ? formatTime(row.online_time) : '-' }}</span>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="190" fixed="right">
          <template #default="{ row }">
            <el-button size="small" type="primary" text @click="openEditDialog(row)">
              <el-icon><Edit /></el-icon> 编辑
            </el-button>
            <el-button
              size="small" type="danger" text
              :disabled="!row.online"
              @click="handleKick(row)"
            >
              <el-icon><SwitchButton /></el-icon> 踢下线
            </el-button>
            <el-button size="small" type="danger" text @click="handleRemove(row)">
              <el-icon><Delete /></el-icon> 删除
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

    <!-- Add Dialog -->
    <el-dialog
      v-model="addDialog.visible"
      title="添加用户"
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
        <el-form-item label="用户名" prop="username">
          <el-input v-model="addDialog.form.username" placeholder="请输入用户名" />
        </el-form-item>
        <el-form-item label="密码" prop="password">
          <el-input v-model="addDialog.form.password" type="password" show-password placeholder="请输入密码" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="addDialog.visible = false">取消</el-button>
        <el-button type="primary" :loading="addDialog.loading" @click="handleAdd">确定添加</el-button>
      </template>
    </el-dialog>

    <!-- Edit Dialog -->
    <el-dialog
      v-model="editDialog.visible"
      title="编辑用户"
      width="440px"
      destroy-on-close
    >
      <el-form
        ref="editFormRef"
        :model="editDialog.form"
        :rules="editRules"
        label-width="80px"
        @submit.prevent
      >
        <el-form-item label="ID">
          <el-input :value="editDialog.form.id" readonly />
        </el-form-item>
        <el-form-item label="用户名" prop="username">
          <el-input v-model="editDialog.form.username" placeholder="请输入用户名" />
        </el-form-item>
        <el-form-item label="新密码" prop="password">
          <el-input v-model="editDialog.form.password" type="password" show-password placeholder="请输入新密码" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="editDialog.visible = false">取消</el-button>
        <el-button type="primary" :loading="editDialog.loading" @click="handleEdit">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Plus, Refresh, Search, Edit, Delete, View, Hide, SwitchButton } from '@element-plus/icons-vue'
import { playerApi } from '@/api'
import type { Player } from '@/types'

// ── State ───────────────────────────────────────────────────────────────────
const loading = ref(false)
const players = ref<Player[]>([])
const searchText = ref('')
const showPasswords = ref<Set<number>>(new Set())

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

function togglePassword(id: number) {
  const s = new Set(showPasswords.value)
  s.has(id) ? s.delete(id) : s.add(id)
  showPasswords.value = s
}

// ── Add ───────────────────────────────────────────────────────────────────────
const addFormRef  = ref<FormInstance>()
const addDialog = reactive({
  visible: false,
  loading: false,
  form: { username: '', password: '' },
})
const addRules: FormRules = {
  username: [{ required: true, message: '请输入用户名', trigger: 'blur' },
             { min: 1, max: 30, message: '用户名 1-30 个字符', trigger: 'blur' }],
  password: [{ required: true, message: '请输入密码', trigger: 'blur' },
             { min: 1, max: 15, message: '密码 1-15 个字符', trigger: 'blur' }],
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
      ElMessage.success('添加成功')
      addDialog.visible = false
      loadData(1)
    } else {
      ElMessage.error(res.data.msg || '添加失败')
    }
  } finally {
    addDialog.loading = false
  }
}

// ── Edit / Change password ─────────────────────────────────────────────────
const editFormRef = ref<FormInstance>()
const editDialog = reactive({
  visible: false,
  loading: false,
  form: { id: 0, username: '', password: '' },
})
const editRules: FormRules = {
  username: [{ required: true, message: '请输入用户名', trigger: 'blur' },
             { min: 1, max: 30, message: '用户名 1-30 个字符', trigger: 'blur' }],
  password: [{ required: true, message: '请输入新密码', trigger: 'blur' },
             { min: 1, max: 15, message: '密码 1-15 个字符', trigger: 'blur' }],
}

function openEditDialog(player: Player) {
  editDialog.form = { id: player.id, username: player.username, password: '' }
  editDialog.visible = true
}

async function handleEdit() {
  const valid = await editFormRef.value?.validate().catch(() => false)
  if (!valid) return
  editDialog.loading = true
  try {
    const res = await playerApi.update(editDialog.form)
    if (res.data.code === 0) {
      ElMessage.success('保存成功')
      editDialog.visible = false
      loadData(pagination.currentPage)
    } else {
      ElMessage.error(res.data.msg || '保存失败')
    }
  } finally {
    editDialog.loading = false
  }
}

// ── Remove ────────────────────────────────────────────────────────────────────
async function handleRemove(player: Player) {
  await ElMessageBox.confirm(
    `确定要删除用户 "${player.username}" 吗？此操作不可恢复。`,
    '删除确认', { type: 'warning', confirmButtonText: '确定删除', cancelButtonText: '取消' }
  )
  const res = await playerApi.remove({ id: player.id })
  if (res.data.code === 0) {
    ElMessage.success('删除成功')
    loadData(pagination.currentPage)
  } else {
    ElMessage.error(res.data.msg || '删除失败')
  }
}

// ── Kick ──────────────────────────────────────────────────────────────────────
async function handleKick(player: Player) {
  await ElMessageBox.confirm(
    `确定要将用户 "${player.username}" 踢下线吗？`,
    '踢下线确认', { type: 'warning', confirmButtonText: '确定', cancelButtonText: '取消' }
  )
  const res = await playerApi.kick({ id: player.id })
  if (res.data.code === 0) {
    ElMessage.success('已踢下线')
    loadData(pagination.currentPage)
  } else {
    ElMessage.error(res.data.msg || '操作失败')
  }
}

// ── Helpers ──────────────────────────────────────────────────────────────────
function formatTime(ts: number): string {
  const d = new Date(ts * 1000)
  const pad = (n: number) => n.toString().padStart(2, '0')
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
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

