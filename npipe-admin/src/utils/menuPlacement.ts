export interface MenuTriggerRect {
  top: number
  right: number
  bottom: number
  left: number
  width?: number
  height?: number
}

export interface PlaceMenuOptions {
  trigger: MenuTriggerRect
  menuWidth: number
  menuHeight: number
  viewportWidth: number
  viewportHeight: number
  gap?: number
  padding?: number
  /** end aligns the menu's right edge to the trigger's right edge (top-right menus). */
  align?: 'start' | 'end'
}

export interface MenuPosition {
  top: number
  left: number
  width: number
  placement: 'bottom' | 'top'
}

export function placeMenu(options: PlaceMenuOptions): MenuPosition {
  const gap = options.gap ?? 6
  const padding = options.padding ?? 8
  const { trigger, viewportWidth, viewportHeight, menuHeight } = options
  const width = Math.min(options.menuWidth, Math.max(0, viewportWidth - padding * 2))

  let left = options.align === 'start' ? trigger.left : trigger.right - width
  const maxLeft = Math.max(padding, viewportWidth - width - padding)
  left = Math.min(Math.max(padding, left), maxLeft)

  const belowTop = trigger.bottom + gap
  const spaceBelow = viewportHeight - padding - belowTop
  const spaceAbove = trigger.top - gap - padding
  const preferBelow = spaceBelow >= menuHeight || spaceBelow >= spaceAbove

  const placement: 'bottom' | 'top' = preferBelow ? 'bottom' : 'top'
  let top = preferBelow ? belowTop : trigger.top - menuHeight - gap

  const minTop = padding
  const maxTop = viewportHeight - padding - menuHeight
  if (maxTop < minTop) {
    top = minTop
  } else {
    top = Math.min(Math.max(minTop, top), maxTop)
  }

  return { top, left, width, placement }
}
