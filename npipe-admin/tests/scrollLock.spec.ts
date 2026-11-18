import { describe, expect, it } from 'vitest'
import {
  addToPadding,
  createScrollLockController,
  measureScrollbarGap,
  type ScrollLockEnv,
} from '@/utils/scrollLock'

function fakeEnv(options: {
  innerWidth?: number
  clientWidth?: number
  bodyPadding?: string
  bodyComputedPadding?: string
  htmlPadding?: string
  htmlComputedPadding?: string
  bodyOverflow?: string
  htmlOverflow?: string
} = {}): ScrollLockEnv {
  const bodyStyle = {
    overflow: options.bodyOverflow ?? '',
    paddingRight: options.bodyPadding ?? '',
  }
  const htmlStyle = {
    overflow: options.htmlOverflow ?? '',
    paddingRight: options.htmlPadding ?? '',
  }
  return {
    innerWidth: options.innerWidth ?? 1280,
    documentClientWidth: options.clientWidth ?? 1280,
    html: {
      style: htmlStyle,
      computedPaddingRight: options.htmlComputedPadding ?? options.htmlPadding ?? '0px',
    },
    body: {
      style: bodyStyle,
      computedPaddingRight: options.bodyComputedPadding ?? options.bodyPadding ?? '0px',
    },
  }
}

describe('measureScrollbarGap', () => {
  it('returns the classic scrollbar width', () => {
    expect(measureScrollbarGap(1280, 1263)).toBe(17)
  })

  it('is zero for overlay scrollbars or pages without overflow', () => {
    expect(measureScrollbarGap(1280, 1280)).toBe(0)
    expect(measureScrollbarGap(390, 390)).toBe(0)
  })

  it('never returns a negative gap', () => {
    expect(measureScrollbarGap(1000, 1020)).toBe(0)
  })
})

describe('addToPadding', () => {
  it('adds the gap to an existing computed padding', () => {
    expect(addToPadding('12px', 17)).toBe('29px')
    expect(addToPadding('0px', 15)).toBe('15px')
    expect(addToPadding('', 15)).toBe('15px')
  })
})

describe('scroll lock controller', () => {
  it('locks overflow on html and body and pads body by the classic scrollbar gap', () => {
    const env = fakeEnv({ innerWidth: 1440, clientWidth: 1425 })
    const lock = createScrollLockController()
    lock.acquire(env)
    expect(env.html.style.overflow).toBe('hidden')
    expect(env.body.style.overflow).toBe('hidden')
    expect(env.body.style.paddingRight).toBe('15px')
    expect(env.html.style.paddingRight).toBe('')
    lock.release()
    expect(env.html.style.overflow).toBe('')
    expect(env.body.style.overflow).toBe('')
    expect(env.body.style.paddingRight).toBe('')
  })

  it('preserves and restores existing padding and overflow', () => {
    const env = fakeEnv({
      innerWidth: 1440,
      clientWidth: 1423,
      bodyPadding: '8px',
      bodyComputedPadding: '8px',
      htmlOverflow: 'auto',
      bodyOverflow: 'scroll',
    })
    const lock = createScrollLockController()
    lock.acquire(env)
    expect(env.body.style.paddingRight).toBe('25px')
    expect(env.body.style.overflow).toBe('hidden')
    lock.release()
    expect(env.body.style.paddingRight).toBe('8px')
    expect(env.body.style.overflow).toBe('scroll')
    expect(env.html.style.overflow).toBe('auto')
  })

  it('does not change padding when there is no visible scrollbar', () => {
    const env = fakeEnv({
      innerWidth: 1280,
      clientWidth: 1280,
      bodyPadding: '10px',
      bodyComputedPadding: '10px',
    })
    const lock = createScrollLockController()
    lock.acquire(env)
    expect(env.body.style.overflow).toBe('hidden')
    expect(env.body.style.paddingRight).toBe('10px')
    lock.release()
    expect(env.body.style.paddingRight).toBe('10px')
  })

  it('keeps the first lock until the last nested overlay releases', () => {
    const env = fakeEnv({ innerWidth: 1600, clientWidth: 1583 })
    const lock = createScrollLockController()
    lock.acquire(env)
    lock.acquire(env)
    expect(lock.count).toBe(2)
    expect(env.body.style.paddingRight).toBe('17px')
    lock.release()
    expect(lock.count).toBe(1)
    expect(env.body.style.overflow).toBe('hidden')
    expect(env.body.style.paddingRight).toBe('17px')
    lock.release()
    expect(lock.count).toBe(0)
    expect(env.body.style.overflow).toBe('')
    expect(env.body.style.paddingRight).toBe('')
  })

  it('is safe when an overlay unmounts after already releasing, or release is called extra times', () => {
    const env = fakeEnv({ innerWidth: 1200, clientWidth: 1183 })
    const lock = createScrollLockController()
    lock.acquire(env)
    lock.release()
    lock.release()
    lock.release()
    expect(lock.count).toBe(0)
    expect(env.body.style.overflow).toBe('')
    expect(env.body.style.paddingRight).toBe('')
  })

  it('models dialog + drawer opening together and closing in any order', () => {
    const env = fakeEnv({ innerWidth: 1920, clientWidth: 1903 })
    const lock = createScrollLockController()
    lock.acquire(env) // Dialog
    lock.acquire(env) // Drawer on top
    lock.release() // Dialog unmounts first
    expect(env.body.style.overflow).toBe('hidden')
    expect(env.body.style.paddingRight).toBe('17px')
    lock.release() // Drawer closes
    expect(env.body.style.overflow).toBe('')
    expect(env.body.style.paddingRight).toBe('')
  })

  it('does not set a fixed body width', () => {
    const env = fakeEnv({ innerWidth: 1440, clientWidth: 1423 }) as ScrollLockEnv & { body: { style: Record<string, string> } }
    const lock = createScrollLockController()
    lock.acquire(env)
    expect(env.body.style.width).toBeUndefined()
    expect(Object.keys(env.body.style)).toEqual(['overflow', 'paddingRight'])
    lock.release()
  })
})
