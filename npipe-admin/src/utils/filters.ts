export const FILTER_PREFIX = 'npipe-filters:'

export function accountScope(role: string, userId: number): string {
  if (role === 'admin') return 'admin'
  if (userId) return `user:${userId}`
  return 'anon'
}

export function filterStorageKey(scope: string, account: string): string {
  return `${FILTER_PREFIX}${account}:${scope}`
}

export function loadFilters<T>(
  scope: string,
  account: string,
  storage: Pick<Storage, 'getItem'> = localStorage,
): T | null {
  try {
    const raw = storage.getItem(filterStorageKey(scope, account))
    if (!raw) return null
    return JSON.parse(raw) as T
  } catch {
    return null
  }
}

export function saveFilters(
  scope: string,
  account: string,
  value: unknown,
  storage: Pick<Storage, 'setItem'> = localStorage,
): void {
  storage.setItem(filterStorageKey(scope, account), JSON.stringify(value))
}

export function clearAllFilters(storage: Pick<Storage, 'getItem' | 'removeItem' | 'length' | 'key'> = localStorage): void {
  const keys: string[] = []
  for (let i = 0; i < storage.length; i += 1) {
    const key = storage.key(i)
    if (key?.startsWith(FILTER_PREFIX)) keys.push(key)
  }
  keys.forEach((key) => storage.removeItem(key))
}
