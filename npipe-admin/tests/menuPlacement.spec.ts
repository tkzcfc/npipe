import { describe, expect, it } from 'vitest'
import { placeMenu } from '@/utils/menuPlacement'

const trigger = {
  top: 16,
  right: 1260,
  bottom: 48,
  left: 1100,
}

describe('placeMenu', () => {
  it('places a top-right account menu below the trigger and right-aligns it', () => {
    const pos = placeMenu({
      trigger,
      menuWidth: 240,
      menuHeight: 196,
      viewportWidth: 1280,
      viewportHeight: 800,
      align: 'end',
    })
    expect(pos.placement).toBe('bottom')
    expect(pos.top).toBe(54)
    expect(pos.left).toBe(1020)
    expect(pos.width).toBe(240)
  })

  it('flips above the trigger when there is not enough room below', () => {
    const pos = placeMenu({
      trigger: { top: 720, right: 1260, bottom: 752, left: 1100 },
      menuWidth: 240,
      menuHeight: 196,
      viewportWidth: 1280,
      viewportHeight: 800,
      align: 'end',
    })
    expect(pos.placement).toBe('top')
    expect(pos.top).toBe(720 - 196 - 6)
    expect(pos.left).toBe(1020)
  })

  it('clamps to the viewport padding instead of overflowing the right or left edge', () => {
    const overflowRight = placeMenu({
      trigger: { top: 20, right: 1278, bottom: 48, left: 1200 },
      menuWidth: 240,
      menuHeight: 160,
      viewportWidth: 1280,
      viewportHeight: 800,
      align: 'end',
    })
    expect(overflowRight.left).toBe(1280 - 240 - 8)

    const overflowLeft = placeMenu({
      trigger: { top: 20, right: 40, bottom: 48, left: 8 },
      menuWidth: 240,
      menuHeight: 160,
      viewportWidth: 1280,
      viewportHeight: 800,
      align: 'end',
    })
    expect(overflowLeft.left).toBe(8)
  })

  it('shrinks the menu and pins it inside a narrow viewport', () => {
    const pos = placeMenu({
      trigger: { top: 10, right: 180, bottom: 42, left: 120 },
      menuWidth: 240,
      menuHeight: 196,
      viewportWidth: 200,
      viewportHeight: 640,
      align: 'end',
    })
    expect(pos.width).toBe(200 - 16)
    expect(pos.left).toBe(8)
    expect(pos.top).toBeGreaterThanOrEqual(8)
    expect(pos.top + 196).toBeLessThanOrEqual(640 - 8)
  })

  it('pins a menu taller than the viewport to the top padding', () => {
    const pos = placeMenu({
      trigger: { top: 400, right: 200, bottom: 432, left: 40 },
      menuWidth: 200,
      menuHeight: 900,
      viewportWidth: 1280,
      viewportHeight: 800,
      align: 'start',
    })
    expect(pos.top).toBe(8)
    expect(pos.left).toBe(40)
  })

  it('aligns to the trigger start when requested', () => {
    const pos = placeMenu({
      trigger: { top: 80, right: 220, bottom: 112, left: 40 },
      menuWidth: 200,
      menuHeight: 100,
      viewportWidth: 1280,
      viewportHeight: 800,
      align: 'start',
    })
    expect(pos.left).toBe(40)
    expect(pos.placement).toBe('bottom')
  })
})
