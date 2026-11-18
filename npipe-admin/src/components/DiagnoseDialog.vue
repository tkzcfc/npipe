<script setup lang="ts">
import { reactive, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { tunnelApi } from '@/api'
import { useAuthStore } from '@/stores/auth'
import { useToastStore } from '@/stores/toast'
import type { Tunnel, TunnelDiagnoseItem, TunnelDiagnoseResponse } from '@/types'
import { clampTunnelEndpoints, diagnoseMessageKey } from '@/utils/tunnel'
import { messageFromResult, messageFromUnknown } from '@/utils/apiMessage'
import Dialog from '@/components/ui/Dialog.vue'
import Button from '@/components/ui/Button.vue'
import StatusBadge from '@/components/StatusBadge.vue'
import Spinner from '@/components/ui/Spinner.vue'

const { t } = useI18n()
const authStore = useAuthStore()
const toast = useToastStore()

const open = ref(false)
const loading = ref(false)
const error = ref('')
const result = reactive<TunnelDiagnoseResponse>({ ok: false, items: [] })

function diagnoseTone(level: TunnelDiagnoseItem['level']) {
  if (level === 'ok') return 'success' as const
  if (level === 'warn') return 'warning' as const
  return 'danger' as const
}

async function run(tunnel: Pick<Tunnel, 'id' | 'source' | 'endpoint' | 'sender' | 'receiver' | 'tunnel_type'>) {
  const auth = { isAdmin: authStore.isAdmin, currentUserId: authStore.currentUserId }
  const ends = clampTunnelEndpoints(tunnel.sender, tunnel.receiver, auth)
  open.value = true
  loading.value = true
  error.value = ''
  result.ok = false
  result.items = []
  try {
    const res = await tunnelApi.diagnose({
      id: tunnel.id || undefined,
      source: tunnel.source,
      endpoint: tunnel.tunnel_type === 2 || tunnel.tunnel_type === 3 ? '' : tunnel.endpoint,
      sender: ends.sender,
      receiver: ends.receiver,
      tunnel_type: tunnel.tunnel_type,
    })
    const data = res.data as TunnelDiagnoseResponse & { code?: number; msg?: string }
    result.ok = data.ok
    result.items = data.items ?? []
    if (!result.items.length && data.code && data.code !== 0) {
      error.value = messageFromResult(data, t, t('common.failed'))
    }
  } catch (err) {
    error.value = messageFromUnknown(err, t, t('common.failed'))
    toast.error(error.value)
  } finally {
    loading.value = false
  }
}

defineExpose({ run })

</script>

<template>
  <Dialog :open="open" :title="t('tunnel.diagnoseTitle')" width="32rem" @update:open="open = $event">
    <Spinner v-if="loading" :label="t('common.loading')" />
    <p v-else-if="error" class="text-sm text-[var(--danger)]">{{ error }}</p>
    <div v-else-if="result.items.length" class="space-y-2">
      <p class="text-sm font-medium">{{ result.ok ? t('tunnel.diagnosePassed') : t('tunnel.diagnoseFailed') }}</p>
      <div v-for="(item, idx) in result.items" :key="idx" class="flex items-start gap-2 text-sm">
        <StatusBadge :tone="diagnoseTone(item.level)" :label="t(`tunnel.diagnoseLevel.${item.level}`)" />
        <span>{{ t(diagnoseMessageKey(item)) }}</span>
      </div>
    </div>
    <p v-else class="text-sm text-muted">{{ t('common.empty') }}</p>
    <template #footer>
      <Button variant="secondary" @click="open = false">{{ t('common.close') }}</Button>
    </template>
  </Dialog>
</template>
