import { describe, expect, it } from 'vitest'
import { applyResolvedTheme, isThemeMode, readThemeMode, resolveTheme } from '../src/utils/theme'

describe('theme helpers', () => {
  it('accepts only known modes', () => {
    expect(isThemeMode('light')).toBe(true)
    expect(isThemeMode('dark')).toBe(true)
    expect(isThemeMode('system')).toBe(true)
    expect(isThemeMode('auto')).toBe(false)
    expect(isThemeMode(null)).toBe(false)
  })

  it('defaults to system when nothing is stored', () => {
    const storage = { getItem: () => null }
    expect(readThemeMode(storage)).toBe('system')
  })

  it('reads the new key first and migrates from the legacy key', () => {
    const map: Record<string, string> = { theme: 'light' }
    expect(readThemeMode({ getItem: (k) => map[k] ?? null })).toBe('light')
    map['npipe-theme'] = 'dark'
    expect(readThemeMode({ getItem: (k) => map[k] ?? null })).toBe('dark')
  })

  it('resolves system from the current preference', () => {
    expect(resolveTheme('system', true)).toBe('dark')
    expect(resolveTheme('system', false)).toBe('light')
    expect(resolveTheme('light', true)).toBe('light')
    expect(resolveTheme('dark', false)).toBe('dark')
  })
})

function fakeRoot() {
  const attrs: Record<string, string> = {}
  const classes = new Set<string>()
  return {
    setAttribute(name: string, value: string) {
      attrs[name] = value
    },
    getAttribute(name: string) {
      return attrs[name] ?? null
    },
    classList: {
      toggle(name: string, force?: boolean) {
        if (force) classes.add(name)
        else classes.delete(name)
      },
      contains(name: string) {
        return classes.has(name)
      },
    },
  }
}

describe('applyResolvedTheme', () => {
  it('sets data-theme and the dark class from the resolved theme', () => {
    const root = fakeRoot()
    applyResolvedTheme('light', 'light', root as unknown as HTMLElement)
    expect(root.getAttribute('data-theme')).toBe('light')
    expect(root.getAttribute('data-theme-mode')).toBe('light')
    expect(root.classList.contains('dark')).toBe(false)
    applyResolvedTheme('dark', 'system', root as unknown as HTMLElement)
    expect(root.getAttribute('data-theme')).toBe('dark')
    expect(root.getAttribute('data-theme-mode')).toBe('system')
    expect(root.classList.contains('dark')).toBe(true)
  })
})
