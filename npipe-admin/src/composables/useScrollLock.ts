import { onBeforeUnmount, watch, type Ref } from 'vue'
import { acquireScrollLock, releaseScrollLock, type ScrollLockEnv } from '@/utils/scrollLock'

export function useScrollLock(active: Ref<boolean>, env?: ScrollLockEnv) {
  let held = false

  function sync(on: boolean) {
    if (on && !held) {
      acquireScrollLock(env)
      held = true
    } else if (!on && held) {
      releaseScrollLock()
      held = false
    }
  }

  watch(active, (on) => sync(on), { immediate: true })
  onBeforeUnmount(() => sync(false))
}
