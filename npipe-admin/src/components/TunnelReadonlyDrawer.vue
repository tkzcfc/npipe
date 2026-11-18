<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAuthStore } from '@/stores/auth'
import type { Tunnel } from '@/types'
import { canManageTunnel, TUNNEL_TYPE_NAMES, tunnelDisplayName } from '@/utils/tunnel'
import { hasProxyAuth } from '@/utils/listQuery'
import { copyText } from '@/utils/clipboard'
import { useToastStore } from '@/stores/toast'
import Drawer from '@/components/ui/Drawer.vue'
import Button from '@/components/ui/Button.vue'
import StatusBadge from '@/components/StatusBadge.vue'
import CopyableText from '@/components/CopyableText.vue'

const emit = defineEmits<{
  edit: [tunnel: Tunnel]
  diagnose: [tunnel: Tunnel]
}>()

const { t } = useI18n()
const authStore = useAuthStore()
const toast = useToastStore()
const open = ref(false)
const tunnel = ref<Tunnel | null>(null)

const canManage = computed(() => {
  if (!tunnel.value) return false
  return canManageTunnel(tunnel.value, { isAdmin: authStore.isAdmin, currentUserId: authStore.currentUserId })
})

const mappingEntries = computed(() => Object.entries(tunnel.value?.custom_mapping ?? {}))

function runtimeTone(item: Tunnel) {
  if (!item.enabled) return 'neutral' as const
  if (item.listen_status === 'failed') return 'danger' as const
  if (item.listen_status === 'offline') return 'warning' as const
  return item.available ? 'success' as const : 'warning' as const
}

function runtimeLabel(item: Tunnel) {
  if (!item.enabled) return t('tunnel.runtime.disabled')
  if (item.listen_status === 'failed') return t('tunnel.listen.failed')
  if (item.listen_status === 'offline') return t('tunnel.listen.offline')
  if (item.listen_status === 'listening') return t('tunnel.listen.listening')
  if (item.available) return t('tunnel.runtime.available')
  return t('tunnel.runtime.waiting')
}

function endpointName(id: number, name: string) {
  if (id === 0) return t('common.server')
  return name || `#${id}`
}

function show(item: Tunnel) {
  tunnel.value = item
  open.value = true
}

async function copy(value: string, ok: string) {
  const result = await copyText(value)
  if (result) toast.success(t('common.copied'))
  else toast.error(t('common.copyFailed'))
  void ok
}

function onEdit() {
  if (!tunnel.value || !canManage.value) return
  emit('edit', tunnel.value)
}

function onDiagnose() {
  if (!tunnel.value) return
  emit('diagnose', tunnel.value)
}

defineExpose({ show })
</script>

<template>
  <Drawer
    :open="open"
    :title="tunnel ? tunnelDisplayName(tunnel) : t('tunnel.readonlyTitle')"
    :description="t('tunnel.readonlySubtitle')"
    width="32rem"
    @update:open="open = $event"
  >
    <div v-if="tunnel" class="space-y-5 text-sm">
      <div class="flex flex-wrap items-center gap-2">
        <StatusBadge :tone="runtimeTone(tunnel)" :label="runtimeLabel(tunnel)" />
        <span class="text-xs text-faint">#{{ tunnel.id }} · {{ TUNNEL_TYPE_NAMES[tunnel.tunnel_type] ?? tunnel.tunnel_type }}</span>
      </div>

      <dl class="space-y-3">
        <div>
          <dt class="text-xs uppercase tracking-wide text-faint">{{ t('tunnel.source') }}</dt>
          <dd class="mt-1"><CopyableText :value="tunnel.source" /></dd>
        </div>
        <div>
          <dt class="text-xs uppercase tracking-wide text-faint">{{ t('tunnel.endpoint') }}</dt>
          <dd class="mt-1"><CopyableText :value="tunnel.endpoint || '—'" /></dd>
        </div>
        <div class="grid grid-cols-2 gap-3">
          <div>
            <dt class="text-xs uppercase tracking-wide text-faint">{{ t('tunnel.senderId') }}</dt>
            <dd class="mt-1">{{ endpointName(tunnel.sender, tunnel.sender_name) }}</dd>
          </div>
          <div>
            <dt class="text-xs uppercase tracking-wide text-faint">{{ t('tunnel.receiverId') }}</dt>
            <dd class="mt-1">{{ endpointName(tunnel.receiver, tunnel.receiver_name) }}</dd>
          </div>
        </div>
        <div class="grid grid-cols-2 gap-3">
          <div>
            <dt class="text-xs uppercase tracking-wide text-faint">{{ t('tunnel.encryption') }}</dt>
            <dd class="mt-1">{{ tunnel.encryption_method || t('tunnel.encryptionNone') }}</dd>
          </div>
          <div>
            <dt class="text-xs uppercase tracking-wide text-faint">{{ t('tunnel.table.compression') }}</dt>
            <dd class="mt-1">{{ tunnel.is_compressed ? t('common.yes') : t('common.no') }}</dd>
          </div>
        </div>
        <div>
          <dt class="text-xs uppercase tracking-wide text-faint">{{ t('common.name') }}</dt>
          <dd class="mt-1 whitespace-pre-wrap">{{ tunnel.description || '—' }}</dd>
        </div>
        <div>
          <dt class="text-xs uppercase tracking-wide text-faint">{{ t('tunnel.authLabel') }}</dt>
          <dd class="mt-1">
            <StatusBadge
              :tone="hasProxyAuth(tunnel) ? 'success' : 'neutral'"
              :label="hasProxyAuth(tunnel) ? t('tunnel.authConfigured') : t('tunnel.authEmpty')"
            />
          </dd>
        </div>
        <div>
          <dt class="text-xs uppercase tracking-wide text-faint">{{ t('tunnel.mapping') }}</dt>
          <dd class="mt-1">
            <p v-if="!mappingEntries.length" class="text-muted">{{ t('tunnel.noMapping') }}</p>
            <ul v-else class="space-y-1">
              <li v-for="[key, value] in mappingEntries" :key="key" class="mono text-[13px]">
                {{ key }} → {{ value }}
              </li>
            </ul>
          </dd>
        </div>
      </dl>

      <div class="flex flex-wrap gap-2 border-t border-line pt-4">
        <Button size="sm" variant="secondary" icon="copy" @click="copy(tunnel.source, 'source')">{{ t('tunnel.copySource') }}</Button>
        <Button v-if="tunnel.endpoint" size="sm" variant="secondary" icon="copy" @click="copy(tunnel.endpoint, 'endpoint')">{{ t('tunnel.copyEndpoint') }}</Button>
        <Button size="sm" variant="secondary" icon="activity" @click="onDiagnose">{{ t('tunnel.diagnose') }}</Button>
        <Button v-if="canManage" size="sm" variant="primary" icon="edit" @click="onEdit">{{ t('tunnel.edit') }}</Button>
      </div>
    </div>
  </Drawer>
</template>
