<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { tunnelApi } from '@/api'
import { useAuthStore } from '@/stores/auth'
import { useToastStore } from '@/stores/toast'
import type { Tunnel, TunnelDiagnoseResponse } from '@/types'
import {
  buildTunnelRequest,
  defaultTunnelForm,
  formFromTunnel,
  isProxyType as isProxy,
  prefillEndpointsForUser,
  snapshotForm,
  diagnoseMessageKey,
  type TunnelForm,
} from '@/utils/tunnel'
import { messageFromResult, messageFromUnknown } from '@/utils/apiMessage'
import { useUnsavedGuard } from '@/composables/useUnsavedGuard'
import Drawer from '@/components/ui/Drawer.vue'
import Button from '@/components/ui/Button.vue'
import Input from '@/components/ui/Input.vue'
import Field from '@/components/ui/Field.vue'
import Select from '@/components/ui/Select.vue'
import Switch from '@/components/ui/Switch.vue'
import Segmented from '@/components/ui/Segmented.vue'
import StatusBadge from '@/components/StatusBadge.vue'
import { cn } from '@/utils/cn'

const emit = defineEmits<{
  saved: []
}>()

const { t } = useI18n()
const authStore = useAuthStore()
const toast = useToastStore()
const auth = () => ({ isAdmin: authStore.isAdmin, currentUserId: authStore.currentUserId })

const visible = ref(false)
const isEdit = ref(false)
const loading = ref(false)
const diagnosing = ref(false)
const error = ref('')
const step = ref(1)
const form = ref<TunnelForm>(defaultTunnelForm())
const formErrors = reactive({ source: '', endpoint: '' })
const snapshot = ref('')
const diagnoseResult = reactive<TunnelDiagnoseResponse>({ ok: false, items: [] })

const isProxyType = computed(() => isProxy(form.value.tunnel_type))
const isDirty = computed(() => visible.value && snapshot.value !== snapshotForm(form.value))
const { confirmDiscard } = useUnsavedGuard(isDirty, () => t('common.unsavedLeave'))

const typeOptions = computed(() => [
  { label: t('tunnel.typeTcp'), value: 0 },
  { label: t('tunnel.typeUdp'), value: 1 },
  { label: t('tunnel.typeSocks5'), value: 2 },
  { label: t('tunnel.typeHttp'), value: 3 },
])

function clearDiagnose() {
  diagnoseResult.ok = false
  diagnoseResult.items = []
}

function openWith(next: TunnelForm, edit: boolean) {
  form.value = next
  isEdit.value = edit
  error.value = ''
  step.value = 1
  formErrors.source = ''
  formErrors.endpoint = ''
  clearDiagnose()
  snapshot.value = snapshotForm(next)
  visible.value = true
}

function openAdd() {
  const ends = authStore.isAdmin
    ? { sender: 0, receiver: 0 }
    : { sender: authStore.currentUserId, receiver: 0 }
  openWith(defaultTunnelForm(ends), false)
}

function openAddForUser(userId: number) {
  if (!authStore.isAdmin && userId !== authStore.currentUserId) return
  openWith(defaultTunnelForm(prefillEndpointsForUser(userId, auth())), false)
}

async function fetchDetail(id: number) {
  const res = await tunnelApi.detail({ id })
  const detail = res.data.tunnel
  if (!detail) {
    toast.error(messageFromResult(res.data as { code?: number; msg?: string }, t, t('tunnel.notFound')))
    return null
  }
  return detail
}

async function openEdit(tunnel: Pick<Tunnel, 'id'>) {
  const detail = await fetchDetail(tunnel.id)
  if (!detail) return
  openWith(formFromTunnel(detail), true)
}

async function openClone(tunnel: Pick<Tunnel, 'id'>) {
  const detail = await fetchDetail(tunnel.id)
  if (!detail) return
  openWith({
    ...formFromTunnel(detail),
    id: 0,
    enabled: true,
    description: detail.description ? `${detail.description} copy` : '',
  }, false)
}

function onTypeChange(value: string | number) {
  form.value.tunnel_type = Number(value)
  error.value = ''
  if (isProxyType.value) form.value.endpoint = ''
  else {
    form.value.username = ''
    form.value.password = ''
  }
}

function validatePath() {
  formErrors.source = form.value.source.trim() ? '' : t('tunnel.validation.sourceRequired')
  formErrors.endpoint = (!isProxyType.value && !form.value.endpoint.trim()) ? t('tunnel.validation.endpointRequired') : ''
  return !formErrors.source && !formErrors.endpoint
}

function goNext() {
  if (step.value === 1 && !validatePath()) return
  step.value = Math.min(3, step.value + 1)
}

async function runDiagnose() {
  diagnosing.value = true
  try {
    const req = buildTunnelRequest(form.value, auth())
    const res = await tunnelApi.diagnose({
      id: isEdit.value ? form.value.id : undefined,
      source: req.source,
      endpoint: req.endpoint,
      sender: req.sender,
      receiver: req.receiver,
      tunnel_type: req.tunnel_type,
    })
    const data = res.data as TunnelDiagnoseResponse & { code?: number; msg?: string }
    if (data.code && data.code !== 0 && !data.items?.length) {
      toast.error(messageFromResult(data, t, t('common.failed')))
      return
    }
    diagnoseResult.ok = data.ok
    diagnoseResult.items = data.items ?? []
  } catch (err) {
    clearDiagnose()
    toast.error(messageFromUnknown(err, t, t('common.failed')))
  } finally {
    diagnosing.value = false
  }
}

async function handleDiagnose() {
  if (!validatePath()) {
    step.value = 1
    return
  }
  await runDiagnose()
}

async function handleSubmit() {
  if (loading.value) return
  if (!validatePath()) {
    step.value = 1
    return
  }
  error.value = ''
  clearDiagnose()
  loading.value = true
  try {
    const req = buildTunnelRequest(form.value, auth())
    const res = isEdit.value ? await tunnelApi.update(req) : await tunnelApi.add(req)
    if (res.data.code === 0) {
      toast.success(isEdit.value ? t('tunnel.saveSuccess') : t('tunnel.addSuccess'))
      snapshot.value = snapshotForm(form.value)
      visible.value = false
      emit('saved')
    } else if (res.data.code === 2) {
      toast.warning(res.data.msg || t('tunnel.savedWithListenWarning'))
      snapshot.value = snapshotForm(form.value)
      visible.value = false
      emit('saved')
    } else {
      error.value = messageFromResult(res.data, t, t('tunnel.submitFailed'))
      toast.error(error.value)
      await runDiagnose()
    }
  } catch (err) {
    error.value = messageFromUnknown(err, t, t('tunnel.submitFailed'))
    toast.error(error.value)
    await runDiagnose()
  } finally {
    loading.value = false
  }
}

function onOpenUpdate(next: boolean) {
  if (!next && !confirmDiscard()) return
  if (!next) visible.value = false
}

function diagnoseTone(level: 'ok' | 'warn' | 'error') {
  if (level === 'ok') return 'success' as const
  if (level === 'warn') return 'warning' as const
  return 'danger' as const
}

defineExpose({ openAdd, openAddForUser, openEdit, openClone })
</script>

<template>
  <Drawer
    :open="visible"
    :title="isEdit ? t('tunnel.editTitle') : t('tunnel.addTitle')"
    width="32rem"
    @update:open="onOpenUpdate"
  >
    <div class="mb-5 flex items-center gap-2 text-xs">
      <span
        v-for="item in 3"
        :key="item"
        :class="cn('flex h-6 w-6 items-center justify-center rounded-full', step === item ? 'bg-accent text-accent-fg' : step > item ? 'bg-accent-dim text-accent' : 'bg-line text-muted')"
      >{{ item }}</span>
      <span class="text-muted">
        {{ step === 1 ? t('tunnel.stepPath') : step === 2 ? t('tunnel.stepEndpoints') : t('tunnel.stepOptions') }}
      </span>
    </div>

    <div v-if="step === 1" class="space-y-4">
      <p class="text-sm text-muted">{{ t('tunnel.stepPathDesc') }}</p>
      <Field :label="t('common.type')">
        <Segmented :model-value="form.tunnel_type" :options="typeOptions" @update:model-value="onTypeChange" />
      </Field>
      <p v-if="isProxyType" class="text-xs text-faint">{{ t('tunnel.proxyHint') }}</p>
      <Field :label="t('tunnel.source')" required :error="formErrors.source">
        <Input v-model="form.source" class="mono" :placeholder="t('tunnel.sourcePlaceholder')" :invalid="!!formErrors.source" />
      </Field>
      <Field v-if="!isProxyType" :label="t('tunnel.endpoint')" required :error="formErrors.endpoint">
        <Input v-model="form.endpoint" class="mono" :placeholder="t('tunnel.endpointPlaceholder')" :invalid="!!formErrors.endpoint" />
      </Field>
    </div>

    <div v-else-if="step === 2" class="space-y-4">
      <p class="text-sm text-muted">{{ t('tunnel.stepEndpointsDesc') }}</p>
      <template v-if="authStore.isAdmin">
        <Field :label="t('tunnel.receiverId')" :hint="t('tunnel.hintServer')">
          <Input v-model="form.receiver" type="number" min="0" />
        </Field>
        <Field :label="t('tunnel.senderId')" :hint="t('tunnel.hintServer')">
          <Input v-model="form.sender" type="number" min="0" />
        </Field>
      </template>
      <template v-else>
        <Field :label="t('tunnel.receiverId')">
          <Select :model-value="form.receiver" @update:model-value="form.receiver = Number($event)">
            <option :value="0">{{ t('tunnel.endpointOption.server') }}</option>
            <option :value="authStore.currentUserId">{{ t('tunnel.endpointOption.self') }}</option>
          </Select>
        </Field>
        <Field :label="t('tunnel.senderId')" :hint="t('tunnel.hintServer')">
          <Input :model-value="authStore.currentUserId" disabled />
        </Field>
      </template>
    </div>

    <div v-else class="space-y-4">
      <p class="text-sm text-muted">{{ t('tunnel.stepOptionsDesc') }}</p>
      <Field :label="t('tunnel.encryption')">
        <Select v-model="form.encryption_method">
          <option value="None">{{ t('tunnel.encryptionNone') }}</option>
          <option value="Xor">Xor</option>
          <option value="Aes128">Aes128</option>
        </Select>
      </Field>
      <Switch v-model="form.is_compressed" :label="t('tunnel.compression')" />
      <Switch v-model="form.enabled" :label="t('tunnel.enabled')" />
      <template v-if="isProxyType">
        <Field :label="t('tunnel.authUser')" :hint="t('common.optional')">
          <Input v-model="form.username" autocomplete="off" />
        </Field>
        <Field :label="t('tunnel.authPass')" :hint="t('common.optional')">
          <Input v-model="form.password" type="password" autocomplete="new-password" />
        </Field>
      </template>
      <Field :label="t('common.description')" :hint="t('common.optional')">
        <Input v-model="form.description" />
      </Field>

      <div v-if="error" class="rounded-lg border border-[var(--danger)]/30 bg-[rgba(214,69,69,.08)] px-3 py-2 text-sm text-[var(--danger)]">{{ error }}</div>

      <div v-if="diagnoseResult.items.length" class="space-y-2 rounded-lg border border-line bg-canvas p-3">
        <p class="text-sm font-medium">{{ diagnoseResult.ok ? t('tunnel.diagnosePassed') : t('tunnel.diagnoseFailed') }}</p>
        <div v-for="(item, idx) in diagnoseResult.items" :key="idx" class="flex items-start gap-2 text-sm">
          <StatusBadge :tone="diagnoseTone(item.level)" :label="t(`tunnel.diagnoseLevel.${item.level}`)" />
          <span>{{ t(diagnoseMessageKey(item)) }}</span>
        </div>
      </div>
    </div>

    <template #footer>
      <div class="flex flex-wrap items-center justify-between gap-2">
        <Button v-if="step > 1" variant="secondary" :disabled="loading" @click="step -= 1">{{ t('common.previous') }}</Button>
        <span v-else />
        <div class="flex flex-wrap gap-2">
          <Button v-if="step === 3" variant="secondary" :disabled="loading" :loading="diagnosing" @click="handleDiagnose">{{ t('tunnel.diagnose') }}</Button>
          <Button v-if="step < 3" variant="primary" @click="goNext">{{ t('common.next') }}</Button>
          <Button v-else variant="primary" :loading="loading" @click="handleSubmit">{{ isEdit ? t('common.save') : t('tunnel.add') }}</Button>
        </div>
      </div>
    </template>
  </Drawer>
</template>
