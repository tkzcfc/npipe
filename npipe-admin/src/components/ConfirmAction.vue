<template>
  <el-dialog
    :model-value="visible"
    @update:model-value="$emit('update:visible', $event)"
    width="440px"
    :close-on-click-modal="false"
    destroy-on-close
    class="confirm-action-dialog"
  >
    <template #header>
      <div class="confirm-action-header">
        <span class="confirm-action-icon" :class="`is-${confirmType}`">
          <el-icon :size="22">
            <Delete v-if="confirmType === 'danger'" />
            <SuccessFilled v-else-if="confirmType === 'success'" />
            <InfoFilled v-else-if="confirmType === 'primary'" />
            <WarningFilled v-else />
          </el-icon>
        </span>
        <span>{{ title }}</span>
      </div>
    </template>

    <div class="confirm-action-body">
      <p class="confirm-action-message">{{ message }}</p>
      <div v-if="details && details.length" class="confirm-action-details">
        <div v-for="d in details" :key="d.label" class="confirm-action-detail-item">
          <span class="detail-label">{{ d.label }}</span>
          <span class="detail-value">{{ d.value }}</span>
        </div>
      </div>
      <div v-if="warningText" class="confirm-action-warning" :class="`is-${confirmType}`">
        <el-icon><Warning /></el-icon>
        <span>{{ warningText }}</span>
      </div>
    </div>

    <template #footer>
      <el-button @click="$emit('update:visible', false)">{{ cancelText }}</el-button>
      <el-button :type="confirmType" :loading="loading" @click="$emit('confirm')">
        <el-icon style="margin-right:4px;">
          <Delete v-if="confirmType === 'danger'" />
          <Check v-else />
        </el-icon>
        {{ confirmText }}
      </el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { Check, Delete, InfoFilled, SuccessFilled, Warning, WarningFilled } from '@element-plus/icons-vue'

withDefaults(defineProps<{
  visible: boolean
  title: string
  message: string
  details?: { label: string; value: string | number }[]
  loading?: boolean
  confirmText?: string
  cancelText?: string
  confirmType?: 'primary' | 'success' | 'warning' | 'danger'
  warningText?: string
}>(), {
  loading: false,
  confirmText: '确认',
  cancelText: '取消',
  confirmType: 'danger',
  warningText: '',
})

defineEmits<{
  'update:visible': [value: boolean]
  confirm: []
}>()
</script>

<style scoped>
.confirm-action-dialog :deep(.el-dialog__header) {
  padding: 20px 24px 0;
}

.confirm-action-dialog :deep(.el-dialog__footer) {
  padding: 16px 24px 22px;
}

.confirm-action-header {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 16px;
  font-weight: 600;
}

.confirm-action-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border-radius: 50%;
}

.confirm-action-icon.is-danger {
  background: rgba(220, 63, 63, 0.12);
  color: var(--danger);
}

.confirm-action-icon.is-warning {
  background: rgba(232, 184, 75, 0.14);
  color: #e8b84b;
}

.confirm-action-icon.is-success {
  background: rgba(53, 200, 124, 0.12);
  color: #35c87c;
}

.confirm-action-icon.is-primary {
  background: rgba(91, 143, 249, 0.14);
  color: var(--accent);
}

.confirm-action-body {
  padding: 8px 24px 0;
}

.confirm-action-message {
  margin: 0 0 16px;
  font-size: 14px;
  line-height: 1.6;
  color: var(--text-primary);
  word-break: break-word;
}

.confirm-action-details {
  background: var(--bg-primary);
  border: 1px solid var(--border-color);
  border-radius: 6px;
  padding: 12px 14px;
  margin-bottom: 14px;
  display: grid;
  gap: 8px;
}

.confirm-action-detail-item {
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
  word-break: break-all;
}

.confirm-action-warning {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  padding: 8px 12px;
  border-radius: 6px;
  margin-bottom: 4px;
}

.confirm-action-warning.is-danger {
  color: var(--danger);
  background: rgba(220, 63, 63, 0.06);
}

.confirm-action-warning.is-warning {
  color: #e8b84b;
  background: rgba(232, 184, 75, 0.08);
}

.confirm-action-warning.is-success {
  color: #35c87c;
  background: rgba(53, 200, 124, 0.08);
}

.confirm-action-warning.is-primary {
  color: var(--accent);
  background: rgba(91, 143, 249, 0.08);
}

.confirm-action-warning .el-icon {
  flex-shrink: 0;
}

@media (max-width: 520px) {
  .confirm-action-dialog {
    width: calc(100vw - 32px) !important;
  }

  .confirm-action-detail-item {
    display: grid;
    gap: 4px;
  }
}
</style>
