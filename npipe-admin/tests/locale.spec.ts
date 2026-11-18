import { describe, expect, it } from 'vitest'
import { htmlLang, isLocaleCode, readLocale } from '../src/utils/locale'

describe('locale helpers', () => {
  it('accepts only supported locales', () => {
    expect(isLocaleCode('zh-CN')).toBe(true)
    expect(isLocaleCode('en-US')).toBe(true)
    expect(isLocaleCode('en')).toBe(false)
  })

  it('falls back to zh-CN when nothing is stored', () => {
    expect(readLocale({ getItem: () => null })).toBe('zh-CN')
  })

  it('prefers npipe-lang over the legacy lang key', () => {
    const map: Record<string, string> = { lang: 'zh-CN', 'npipe-lang': 'en-US' }
    expect(readLocale({ getItem: (k) => map[k] ?? null })).toBe('en-US')
  })

  it('maps html lang attributes', () => {
    expect(htmlLang('en-US')).toBe('en')
    expect(htmlLang('zh-CN')).toBe('zh-CN')
  })
})

import zh from '../src/locales/zh-CN'
import en from '../src/locales/en-US'

function keys(value: unknown, prefix = ''): string[] {
  if (!value || typeof value !== 'object') return [prefix]
  return Object.entries(value as Record<string, unknown>).flatMap(([key, child]) =>
    keys(child, prefix ? `${prefix}.${key}` : key),
  )
}

describe('locale coverage', () => {
  it('keeps zh-CN and en-US key trees in sync', () => {
    expect(keys(en).sort()).toEqual(keys(zh).sort())
  })
})
