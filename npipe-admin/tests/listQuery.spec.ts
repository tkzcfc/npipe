import { describe, expect, it } from 'vitest'
import {
  LIST_QUERY_VERSION,
  clampQuery,
  hasProxyAuth,
  isApiErrorPayload,
  resolveAssociatedTunnelSource,
  supportsListQuery,
} from '@/utils/listQuery'

describe('supportsListQuery', () => {
  it('requires the constant query_version and a successful payload', () => {
    expect(supportsListQuery({ query_version: LIST_QUERY_VERSION, tunnels: [] })).toBe(true)
    expect(supportsListQuery({ query_version: LIST_QUERY_VERSION, players: [], total_count: 0 })).toBe(true)
    expect(supportsListQuery({ tunnels: [] })).toBe(false)
    expect(supportsListQuery({ query_version: 2 })).toBe(false)
    expect(supportsListQuery({ filtered: true })).toBe(false)
    expect(supportsListQuery({ query_version: 1, code: -1, msg: 'invalid' })).toBe(false)
    expect(supportsListQuery({ query_version: 1, code: 403 })).toBe(false)
    expect(supportsListQuery(undefined)).toBe(false)
  })
})

describe('resolveAssociatedTunnelSource', () => {
  it('uses the list only when the server advertises query capability', () => {
    expect(resolveAssociatedTunnelSource({ query_version: 1, tunnels: [{ id: 1 }] })).toBe('server')
  })

  it('falls back to player_detail when an old server ignores user_id', () => {
    expect(resolveAssociatedTunnelSource({ tunnels: [{ id: 99, sender: 1 }] })).toBe('legacy-detail')
    expect(resolveAssociatedTunnelSource({ code: 403, msg: 'Forbidden' })).toBe('legacy-detail')
  })
})

describe('helpers', () => {
  it('clamps and trims search text', () => {
    expect(clampQuery('  alice  ')).toBe('alice')
    expect(clampQuery('')).toBe('')
    expect(clampQuery('x'.repeat(80)).length).toBe(64)
  })

  it('detects proxy auth without exposing a password field', () => {
    expect(hasProxyAuth({ username: 'proxy' })).toBe(true)
    expect(hasProxyAuth({ username: '' })).toBe(false)
    expect(hasProxyAuth({})).toBe(false)
  })

  it('detects API error payloads', () => {
    expect(isApiErrorPayload({ code: -1 })).toBe(true)
    expect(isApiErrorPayload({ query_version: 1 })).toBe(false)
  })
})
