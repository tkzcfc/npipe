import { describe, expect, it } from 'vitest'
import { accountScope, clearAllFilters, FILTER_PREFIX, loadFilters, saveFilters } from '../src/utils/filters'

function memoryStorage(initial: Record<string, string> = {}) {
  const map = new Map(Object.entries(initial))
  const storage = {
    get length() { return map.size },
    key(index: number) { return [...map.keys()][index] ?? null },
    getItem(key: string) { return map.get(key) ?? null },
    setItem(key: string, value: string) { map.set(key, value) },
    removeItem(key: string) { map.delete(key) },
  }
  return { map, storage }
}

describe('filter persistence', () => {
  it('isolates saved filters by account', () => {
    const { storage } = memoryStorage()
    saveFilters('tunnels', accountScope('admin', 0), { search: 'admin-only' }, storage)
    saveFilters('tunnels', accountScope('user', 42), { search: 'user-42' }, storage)
    expect(loadFilters('tunnels', 'admin', storage)).toEqual({ search: 'admin-only' })
    expect(loadFilters('tunnels', 'user:42', storage)).toEqual({ search: 'user-42' })
    expect(loadFilters('tunnels', 'user:7', storage)).toBeNull()
  })

  it('clears only the filter namespace on logout', () => {
    const { map, storage } = memoryStorage({
      theme: 'dark',
      [`${FILTER_PREFIX}admin:tunnels`]: '{"search":"x"}',
      [`${FILTER_PREFIX}user:42:players`]: '{"search":"y"}',
    })
    clearAllFilters(storage)
    expect(map.has('theme')).toBe(true)
    expect([...map.keys()].some((key) => key.startsWith(FILTER_PREFIX))).toBe(false)
  })

  it('uses distinct scopes for the config admin and regular users', () => {
    expect(accountScope('admin', 0)).toBe('admin')
    expect(accountScope('user', 15)).toBe('user:15')
  })
})
