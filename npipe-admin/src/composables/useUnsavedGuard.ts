import { onBeforeUnmount, onMounted, type Ref } from 'vue'
import { onBeforeRouteLeave } from 'vue-router'

export function useUnsavedGuard(isDirty: Ref<boolean>, message: () => string) {
  function onBeforeUnload(event: BeforeUnloadEvent) {
    if (!isDirty.value) return
    event.preventDefault()
    event.returnValue = ''
  }

  onMounted(() => window.addEventListener('beforeunload', onBeforeUnload))
  onBeforeUnmount(() => window.removeEventListener('beforeunload', onBeforeUnload))

  onBeforeRouteLeave(() => {
    if (!isDirty.value) return true
    return window.confirm(message())
  })

  function confirmDiscard(): boolean {
    if (!isDirty.value) return true
    return window.confirm(message())
  }

  return { confirmDiscard }
}
