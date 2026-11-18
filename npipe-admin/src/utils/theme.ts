export type ThemeMode = 'light' | 'dark' | 'system'
export type ResolvedTheme = 'light' | 'dark'

export const THEME_STORAGE_KEY = 'npipe-theme'
export const THEME_LEGACY_KEY = 'theme'

export function isThemeMode(value: unknown): value is ThemeMode {
  return value === 'light' || value === 'dark' || value === 'system'
}

export function readThemeMode(
  storage: Pick<Storage, 'getItem'>,
  keys: string[] = [THEME_STORAGE_KEY, THEME_LEGACY_KEY],
): ThemeMode {
  for (const key of keys) {
    const value = storage.getItem(key)
    if (isThemeMode(value)) return value
  }
  return 'system'
}

export function resolveTheme(mode: ThemeMode, prefersDark: boolean): ResolvedTheme {
  if (mode === 'system') return prefersDark ? 'dark' : 'light'
  return mode
}

export function applyResolvedTheme(resolved: ResolvedTheme, mode: ThemeMode, root: HTMLElement = document.documentElement) {
  root.setAttribute('data-theme', resolved)
  root.setAttribute('data-theme-mode', mode)
  root.classList.toggle('dark', resolved === 'dark')
}
