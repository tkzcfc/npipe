export interface StyleBag {
  overflow: string
  paddingRight: string
}

export interface LockTarget {
  style: StyleBag
  computedPaddingRight: string
}

export interface ScrollLockEnv {
  innerWidth: number
  documentClientWidth: number
  html: LockTarget
  body: LockTarget
}

export function measureScrollbarGap(innerWidth: number, clientWidth: number): number {
  return Math.max(0, innerWidth - clientWidth)
}

export function addToPadding(currentPadding: string, gap: number): string {
  const current = Number.parseFloat(currentPadding) || 0
  return `${current + gap}px`
}

function snapshot(target: LockTarget): StyleBag {
  return {
    overflow: target.style.overflow,
    paddingRight: target.style.paddingRight,
  }
}

export function createDomScrollLockEnv(): ScrollLockEnv {
  const html = document.documentElement
  const body = document.body
  return {
    get innerWidth() {
      return window.innerWidth
    },
    get documentClientWidth() {
      return html.clientWidth
    },
    html: {
      style: html.style,
      get computedPaddingRight() {
        return getComputedStyle(html).paddingRight
      },
    },
    body: {
      style: body.style,
      get computedPaddingRight() {
        return getComputedStyle(body).paddingRight
      },
    },
  }
}

export function createScrollLockController(getDefaultEnv?: () => ScrollLockEnv) {
  let count = 0
  let applied: { env: ScrollLockEnv; html: StyleBag; body: StyleBag } | null = null

  function acquire(env?: ScrollLockEnv) {
    count += 1
    if (count !== 1) return

    const next = env ?? getDefaultEnv?.() ?? createDomScrollLockEnv()
    const gap = measureScrollbarGap(next.innerWidth, next.documentClientWidth)
    applied = {
      env: next,
      html: snapshot(next.html),
      body: snapshot(next.body),
    }
    next.html.style.overflow = 'hidden'
    next.body.style.overflow = 'hidden'
    if (gap > 0) {
      next.body.style.paddingRight = addToPadding(next.body.computedPaddingRight, gap)
    }
  }

  function release() {
    if (count === 0) return
    count -= 1
    if (count !== 0) return

    const current = applied
    applied = null
    if (!current) return
    current.env.html.style.overflow = current.html.overflow
    current.env.html.style.paddingRight = current.html.paddingRight
    current.env.body.style.overflow = current.body.overflow
    current.env.body.style.paddingRight = current.body.paddingRight
  }

  return {
    acquire,
    release,
    get count() {
      return count
    },
  }
}

const globalScrollLock = createScrollLockController()

export function acquireScrollLock(env?: ScrollLockEnv) {
  globalScrollLock.acquire(env)
}

export function releaseScrollLock() {
  globalScrollLock.release()
}

export function getScrollLockCount() {
  return globalScrollLock.count
}

/** Test helper: drain the singleton without touching styles if already restored. */
export function resetGlobalScrollLockForTests() {
  while (globalScrollLock.count > 0) {
    globalScrollLock.release()
  }
}
