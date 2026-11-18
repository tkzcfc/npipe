import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'

const cssPath = resolve(dirname(fileURLToPath(import.meta.url)), '../src/styles/index.css')

function parseBlock(css: string, heading: RegExp): Record<string, string> {
  const match = css.match(heading)
  if (!match || match.index === undefined) throw new Error(`theme block not found: ${heading}`)
  const start = css.indexOf('{', match.index)
  const end = css.indexOf('}', start)
  const body = css.slice(start + 1, end)
  const tokens: Record<string, string> = {}
  for (const line of body.split(';')) {
    const item = line.trim()
    if (!item.startsWith('--')) continue
    const colon = item.indexOf(':')
    tokens[item.slice(0, colon).trim()] = item.slice(colon + 1).trim()
  }
  return tokens
}

function hexLuminance(hex: string): number {
  const raw = hex.replace('#', '').trim()
  if (!/^[\da-fA-F]{6}$/.test(raw)) throw new Error(`expected 6-digit hex, got ${hex}`)
  const ch = (i: number) => {
    const v = Number.parseInt(raw.slice(i, i + 2), 16) / 255
    return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4
  }
  return 0.2126 * ch(0) + 0.7152 * ch(2) + 0.0722 * ch(4)
}

function contrastRatio(a: string, b: string): number {
  const l1 = hexLuminance(a)
  const l2 = hexLuminance(b)
  const [hi, lo] = l1 > l2 ? [l1, l2] : [l2, l1]
  return (hi + 0.05) / (lo + 0.05)
}

describe('theme tokens', () => {
  const css = readFileSync(cssPath, 'utf8')
  const light = parseBlock(css, /:root,\s*:root\[data-theme='light'\]/)
  const dark = parseBlock(css, /:root\[data-theme='dark'\],\s*\.dark/)

  it('uses a light sidebar in the light theme, not a leftover dark ink panel', () => {
    expect(hexLuminance(light['--bg-sidebar'])).toBeGreaterThan(0.8)
    expect(hexLuminance(light['--text-sidebar'])).toBeLessThan(0.2)
    expect(hexLuminance(light['--text-sidebar-muted'])).toBeLessThan(0.45)
    expect(contrastRatio(light['--bg-sidebar'], light['--text-sidebar'])).toBeGreaterThanOrEqual(7)
    expect(contrastRatio(light['--bg-sidebar'], light['--text-sidebar-muted'])).toBeGreaterThanOrEqual(4.5)
    expect(light['--bg-sidebar']).toBe(light['--bg-card'])
    expect(light['--text-sidebar']).toBe(light['--text'])
    expect(light['--bg-sidebar-hover'].startsWith('rgba(255')).toBe(false)
  })

  it('keeps the dark sidebar as a dark ink panel', () => {
    expect(hexLuminance(dark['--bg-sidebar'])).toBeLessThan(0.05)
    expect(hexLuminance(dark['--text-sidebar'])).toBeGreaterThan(0.8)
    expect(contrastRatio(dark['--bg-sidebar'], dark['--text-sidebar'])).toBeGreaterThanOrEqual(7)
    expect(dark['--bg']).toBe('#0b0f14')
    expect(dark['--accent']).toBe('#2ee6a6')
  })
})
