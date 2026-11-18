import type { Tunnel, TunnelDetail, TunnelDiagnoseItem, TunnelMutateRequest } from '@/types'

export const TUNNEL_TYPE_NAMES: Record<number, string> = { 0: 'TCP', 1: 'UDP', 2: 'SOCKS5', 3: 'HTTP' }

export interface AuthSnapshot {
  isAdmin: boolean
  currentUserId: number
}

export interface TunnelForm {
  id: number
  source: string
  endpoint: string
  tunnel_type: number
  sender: number
  receiver: number
  username: string
  password: string
  encryption_method: string
  is_compressed: boolean
  enabled: boolean
  description: string
  custom_mapping: Record<string, string>
}

export function canManageTunnel(tunnel: Pick<Tunnel, 'sender' | 'receiver'>, auth: AuthSnapshot): boolean {
  return auth.isAdmin || (tunnel.sender === auth.currentUserId && (tunnel.receiver === 0 || tunnel.receiver === auth.currentUserId))
}

export function clampTunnelEndpoints(
  sender: number,
  receiver: number,
  auth: AuthSnapshot,
): { sender: number; receiver: number } {
  if (auth.isAdmin) return { sender, receiver }
  return {
    sender: auth.currentUserId,
    receiver: receiver === auth.currentUserId ? auth.currentUserId : 0,
  }
}

/** Prefill for “create tunnel for this user”. Non-admins can only create as themselves. */
export function prefillEndpointsForUser(userId: number, auth: AuthSnapshot): { sender: number; receiver: number } {
  if (!auth.isAdmin) {
    return { sender: auth.currentUserId, receiver: 0 }
  }
  return { sender: userId, receiver: 0 }
}

export function defaultTunnelForm(endpoints?: { sender: number; receiver: number }): TunnelForm {
  return {
    id: 0,
    source: '',
    endpoint: '',
    tunnel_type: 0,
    sender: endpoints?.sender ?? 0,
    receiver: endpoints?.receiver ?? 0,
    username: '',
    password: '',
    encryption_method: 'Xor',
    is_compressed: true,
    enabled: true,
    description: '',
    custom_mapping: {},
  }
}

export function formFromTunnel(tunnel: TunnelDetail): TunnelForm {
  return {
    id: tunnel.id,
    source: tunnel.source,
    endpoint: tunnel.endpoint,
    tunnel_type: tunnel.tunnel_type,
    sender: tunnel.sender,
    receiver: tunnel.receiver,
    username: tunnel.username,
    password: tunnel.password,
    encryption_method: tunnel.encryption_method || 'None',
    is_compressed: tunnel.is_compressed,
    enabled: tunnel.enabled,
    description: tunnel.description,
    custom_mapping: tunnel.custom_mapping || {},
  }
}

export function isProxyType(tunnelType: number): boolean {
  return tunnelType === 2 || tunnelType === 3
}

export function buildTunnelRequest(form: TunnelForm, auth: AuthSnapshot): TunnelMutateRequest {
  const { sender, receiver } = clampTunnelEndpoints(Number(form.sender) || 0, Number(form.receiver) || 0, auth)
  const proxy = isProxyType(form.tunnel_type)
  return {
    id: form.id,
    source: form.source,
    endpoint: proxy ? '' : form.endpoint,
    enabled: form.enabled ? 1 : 0,
    sender,
    receiver,
    description: form.description,
    tunnel_type: form.tunnel_type,
    password: proxy ? form.password : '',
    username: proxy ? form.username : '',
    is_compressed: form.is_compressed ? 1 : 0,
    encryption_method: form.encryption_method,
    custom_mapping: form.custom_mapping || {},
  }
}

export function tunnelDisplayName(tunnel: Pick<Tunnel, 'description' | 'source' | 'tunnel_type'>): string {
  const desc = tunnel.description?.trim()
  if (desc) return desc
  const type = TUNNEL_TYPE_NAMES[tunnel.tunnel_type] ?? String(tunnel.tunnel_type)
  return tunnel.source ? `${type} ${tunnel.source}` : type
}

export function diagnoseMessageKey(item: Pick<TunnelDiagnoseItem, 'key' | 'level'>): string {
  return `tunnel.diagnoseMessage.${item.key}.${item.level}`
}

export function snapshotForm(form: TunnelForm): string {
  return JSON.stringify(form)
}
