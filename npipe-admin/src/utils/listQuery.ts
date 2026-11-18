export const LIST_QUERY_VERSION = 1

export interface QueryVersioned {
  query_version?: number
  code?: number
}

export function supportsListQuery(data: QueryVersioned | null | undefined): boolean {
  if (!data || typeof data !== 'object') return false
  if (typeof data.code === 'number' && data.code !== 0) return false
  return data.query_version === LIST_QUERY_VERSION
}

export function isApiErrorPayload(data: QueryVersioned | null | undefined): boolean {
  return typeof data?.code === 'number' && data.code !== 0
}

export function clampQuery(raw: string, max = 64): string {
  const trimmed = raw.trim()
  if (!trimmed) return ''
  return [...trimmed].slice(0, max).join('')
}

export function hasProxyAuth(tunnel: { username?: string }): boolean {
  return Boolean(tunnel.username && tunnel.username.length > 0)
}

/**
 * Decide how to populate a user's associated tunnels.
 * If the server does not advertise query_version, the list must be discarded
 * because user_id may have been ignored — never treat an unfiltered page as
 * this user's tunnels.
 */
export function resolveAssociatedTunnelSource(
  listData: (QueryVersioned & { tunnels?: unknown[] }) | null | undefined,
): 'server' | 'legacy-detail' {
  if (supportsListQuery(listData)) return 'server'
  return 'legacy-detail'
}
