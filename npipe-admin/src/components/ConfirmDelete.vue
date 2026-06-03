<template>
  <el-dialog
    :model-value="visible"
    @update:model-value="$emit('update:visible', $event)"
    width="440px"
    :close-on-click-modal="false"
    destroy-on-close
    class="confirm-delete-dialog"
  >
    <template #header>
      <div class="confirm-delete-header">
        <span class="confirm-delete-icon">
          <el-icon :size="22"><WarningFilled /></el-icon>
        </span>
        <span>{{ title }}</span>
      </div>
    </template>

    <div class="confirm-delete-body">
      <p class="confirm-delete-message">{{ message }}</p>
      <div v-if="details && details.length" class="confirm-delete-details">
        <div v-for="d in details" :key="d.label" class="confirm-delete-detail-item">
          <span class="detail-label">{{ d.label }}</span>
          <span class="detail-value">{{ d.value }}</span>
        </div>
      </div>
      <div class="confirm-delete-warning">
        <el-icon><Warning /></el-icon>
        <span>{{ $t('common.irreversible') }}</span>
      </div>
    </div>

    <template #footer>
      <el-button @click="$emit('update:visible', false)">{{ cancelText }}</el-button>
      <el-button type="danger" :loading="loading" @click="$emit('confirm')">
        <el-icon style="margin-right:4px;"><Delete /></el-icon>
        {{ confirmText }}
      </el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { WarningFilled, Warning, Delete } from '@element-plus/icons-vue'

defineProps<{
  visible: boolean
  title: string
  message: string
  details?: { label: string; value: string }[]
  loading?: boolean
  confirmText?: string
  cancelText?: string
}>()

defineEmits<{
  'update:visible': [value: boolean]
  confirm: []
}>()
</script>

<style scoped>
.confirm-delete-dialog :deep(.el-dialog__header) {
  padding: 20px 24px 0;
}

.confirm-delete-header {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 16px;
  font-weight: 600;
}

.confirm-delete-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border-radius: 50%;
  background: rgba(220, 63, 63, 0.12);
  color: var(--danger);
}

.confirm-delete-body {
  padding: 8px 24px 0;
}

.confirm-delete-message {
  margin: 0 0 16px;
  font-size: 14px;
  line-height: 1.6;
  color: var(--text-primary);
}

.confirm-delete-details {
  background: var(--bg-primary);
  border: 1px solid var(--border-color);
  border-radius: 6px;
  padding: 12px 14px;
  margin-bottom: 14px;
  display: grid;
  gap: 8px;
}

.confirm-delete-detail-item {
  display: flex;
  gap: 12px;
  font-size: 13px;
}

.detail-label {
  color: var(--text-muted);
  flex-shrink: 0;
  min-width: 50px;
}

.detail-value {
  color: var(--text-primary);
  font-family: 'JetBrains Mono', Consolas, monospace;
  font-size: 12px;
}

.confirm-delete-warning {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--danger);
  padding: 8px 12px;
  background: rgba(220, 63, 63, 0.06);
  border-radius: 6px;
  margin-bottom: 4px;
}

.confirm-delete-warning .el-icon {
  flex-shrink: 0;
}
</style>
