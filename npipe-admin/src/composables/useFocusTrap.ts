import { nextTick, onBeforeUnmount, watch, type Ref } from 'vue'

const SELECTOR = 'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])'

export function useFocusTrap(container: Ref<HTMLElement | null>, active: Ref<boolean>) {
  let previous: HTMLElement | null = null

  function getFocusable() {
    return Array.from(container.value?.querySelectorAll<HTMLElement>(SELECTOR) ?? [])
      .filter((el) => !el.hasAttribute('disabled') && el.tabIndex !== -1 && el.offsetParent !== null)
  }

  function onKey(event: KeyboardEvent) {
    if (event.key === 'Tab') {
      const items = getFocusable()
      if (!items.length) return
      const first = items[0]
      const last = items[items.length - 1]
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault()
        last.focus()
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault()
        first.focus()
      }
    }
  }

  watch(active, async (open) => {
    if (open) {
      previous = document.activeElement as HTMLElement | null
      await nextTick()
      const items = getFocusable()
      items[0]?.focus()
      document.addEventListener('keydown', onKey)
    } else {
      document.removeEventListener('keydown', onKey)
      previous?.focus?.()
    }
  }, { immediate: true })

  onBeforeUnmount(() => document.removeEventListener('keydown', onKey))
}
