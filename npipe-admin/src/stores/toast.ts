import { defineStore } from 'pinia'
import { ref } from 'vue'

export type ToastKind = 'success' | 'error' | 'warning' | 'info'

export interface ToastItem {
  id: number
  kind: ToastKind
  message: string
}

let seed = 1

export const useToastStore = defineStore('toast', () => {
  const items = ref<ToastItem[]>([])

  function push(kind: ToastKind, message: string, ttl = 4200) {
    const id = seed++
    items.value = [...items.value, { id, kind, message }]
    window.setTimeout(() => dismiss(id), ttl)
  }

  function dismiss(id: number) {
    items.value = items.value.filter((item) => item.id !== id)
  }

  return {
    items,
    push,
    dismiss,
    success: (message: string) => push('success', message),
    error: (message: string) => push('error', message, 6400),
    warning: (message: string) => push('warning', message, 8000),
    info: (message: string) => push('info', message),
  }
})
