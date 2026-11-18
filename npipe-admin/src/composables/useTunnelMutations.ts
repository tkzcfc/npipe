import { reactive, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { tunnelApi } from '@/api'
import { useToastStore } from '@/stores/toast'
import type { Tunnel } from '@/types'
import { TUNNEL_TYPE_NAMES, tunnelDisplayName } from '@/utils/tunnel'
import { messageFromResult, messageFromUnknown } from '@/utils/apiMessage'

export function useTunnelMutations(onChanged: () => void) {
  const { t } = useI18n()
  const toast = useToastStore()
  const toggling = ref<Set<number>>(new Set())

  const confirm = reactive({
    open: false,
    loading: false,
    kind: '' as 'status' | 'delete' | '',
    tunnel: null as Tunnel | null,
    title: '',
    message: '',
    confirmText: '',
    tone: 'warning' as 'warning' | 'success' | 'danger',
    details: [] as { label: string; value: string | number }[],
  })

  function detailsOf(tunnel: Tunnel) {
    return [
      { label: t('common.id'), value: tunnel.id },
      { label: t('tunnel.table.name'), value: tunnelDisplayName(tunnel) },
      { label: t('tunnel.source'), value: tunnel.source || '—' },
      { label: t('tunnel.endpoint'), value: tunnel.endpoint || '—' },
      { label: t('common.type'), value: TUNNEL_TYPE_NAMES[tunnel.tunnel_type] ?? String(tunnel.tunnel_type) },
    ]
  }

  function requestToggle(tunnel: Tunnel) {
    confirm.kind = 'status'
    confirm.tunnel = tunnel
    confirm.title = t(tunnel.enabled ? 'tunnel.disableTitle' : 'tunnel.enableTitle')
    confirm.message = t(tunnel.enabled ? 'tunnel.disableConfirm' : 'tunnel.enableConfirm', {
      name: tunnelDisplayName(tunnel),
    })
    confirm.confirmText = t(tunnel.enabled ? 'common.disable' : 'common.enable')
    confirm.tone = tunnel.enabled ? 'warning' : 'success'
    confirm.details = detailsOf(tunnel)
    confirm.loading = false
    confirm.open = true
  }

  function requestRemove(tunnel: Tunnel) {
    confirm.kind = 'delete'
    confirm.tunnel = tunnel
    confirm.title = t('tunnel.deleteTitle')
    confirm.message = t('tunnel.deleteConfirm', { desc: tunnelDisplayName(tunnel) })
    confirm.confirmText = t('common.delete')
    confirm.tone = 'danger'
    confirm.details = detailsOf(tunnel)
    confirm.loading = false
    confirm.open = true
  }

  async function confirmAction() {
    const target = confirm.tunnel
    if (!target || !confirm.kind || confirm.loading) return
    confirm.loading = true
    if (confirm.kind === 'status') {
      const next = new Set(toggling.value)
      next.add(target.id)
      toggling.value = next
    }
    try {
      if (confirm.kind === 'status') {
        const res = await tunnelApi.updateStatus({ id: target.id, enabled: target.enabled ? 0 : 1 })
        if (res.data.code === 0) {
          toast.success(t('tunnel.toggleSuccess'))
          confirm.open = false
          onChanged()
        } else if (res.data.code === 2) {
          toast.warning(res.data.msg || t('tunnel.savedWithListenWarning'))
          confirm.open = false
          onChanged()
        } else {
          toast.error(messageFromResult(res.data, t, t('common.failed')))
        }
        return
      }
      const res = await tunnelApi.remove({ id: target.id })
      if (res.data.code === 0) {
        toast.success(t('tunnel.deleteSuccess'))
        confirm.open = false
        onChanged()
      } else {
        toast.error(messageFromResult(res.data, t, t('common.failed')))
      }
    } catch (err) {
      toast.error(messageFromUnknown(err, t, t('common.failed')))
    } finally {
      confirm.loading = false
      if (target) {
        const rest = new Set(toggling.value)
        rest.delete(target.id)
        toggling.value = rest
      }
    }
  }

  return { toggling, confirm, requestToggle, requestRemove, confirmAction }
}
