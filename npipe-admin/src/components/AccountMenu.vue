<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAuthStore } from '@/stores/auth'
import { cn } from '@/utils/cn'
import Icon from '@/components/ui/Icon.vue'
import { placeMenu } from '@/utils/menuPlacement'

const emit = defineEmits<{
  logout: []
  account: []
  loginLogs: []
}>()

const { t } = useI18n()
const auth = useAuthStore()
const open = ref(false)
const trigger = ref<HTMLElement | null>(null)
const menu = ref<HTMLElement | null>(null)
const style = ref<Record<string, string>>({})

const roleLabel = computed(() => (auth.isAdmin ? t('layout.admin') : t('layout.user')))
const initial = computed(() => (auth.displayName || 'U').slice(0, 1).toUpperCase())

function place() {
  const rect = trigger.value?.getBoundingClientRect()
  if (!rect) return
  const measured = menu.value?.getBoundingClientRect().height
  const pos = placeMenu({
    trigger: { top: rect.top, right: rect.right, bottom: rect.bottom, left: rect.left },
    menuWidth: 240,
    menuHeight: measured && measured > 0 ? measured : (auth.isAdmin ? 148 : 192),
    viewportWidth: window.innerWidth,
    viewportHeight: window.innerHeight,
    align: 'end',
  })
  style.value = {
    position: 'fixed',
    top: `${pos.top}px`,
    left: `${pos.left}px`,
    width: `${pos.width}px`,
    zIndex: '70',
  }
}

function close() {
  open.value = false
}

function toggle() {
  open.value = !open.value
}

function onDoc(event: MouseEvent) {
  const target = event.target as Node
  if (trigger.value?.contains(target) || menu.value?.contains(target)) return
  close()
}

function onKey(event: KeyboardEvent) {
  if (!open.value) return
  if (event.key === 'Escape') {
    event.preventDefault()
    close()
    trigger.value?.querySelector('button')?.focus()
    return
  }
  if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return
  event.preventDefault()
  const buttons = Array.from(menu.value?.querySelectorAll<HTMLButtonElement>('button[data-item]') ?? [])
  if (!buttons.length) return
  const index = buttons.findIndex((el) => el === document.activeElement)
  const next = event.key === 'ArrowDown'
    ? (index + 1) % buttons.length
    : (index <= 0 ? buttons.length - 1 : index - 1)
  buttons[next]?.focus()
}

function addListeners() {
  document.addEventListener('mousedown', onDoc)
  document.addEventListener('keydown', onKey)
  window.addEventListener('resize', place)
  window.addEventListener('scroll', place, true)
}

function removeListeners() {
  document.removeEventListener('mousedown', onDoc)
  document.removeEventListener('keydown', onKey)
  window.removeEventListener('resize', place)
  window.removeEventListener('scroll', place, true)
}

watch(open, async (value) => {
  if (value) {
    await nextTick()
    place()
    await nextTick()
    place()
    menu.value?.querySelector<HTMLButtonElement>('button[data-item]')?.focus()
    addListeners()
  } else {
    removeListeners()
  }
})

onBeforeUnmount(() => {
  removeListeners()
  close()
})

function goAccount() {
  emit('account')
  close()
}

function goLoginLogs() {
  emit('loginLogs')
  close()
}

function goLogout() {
  emit('logout')
  close()
}
</script>

<template>
  <div ref="trigger" class="relative shrink-0">
    <button
      type="button"
      class="flex max-w-[13rem] items-center gap-2 rounded-lg py-1 pl-1 pr-1.5 text-left text-ink transition-colors hover:bg-canvas md:pr-2"
      :aria-haspopup="true"
      :aria-expanded="open"
      :aria-label="t('layout.accountMenu')"
      :title="auth.displayName"
      @click="toggle"
    >
      <span class="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-accent-dim text-xs font-semibold text-accent">
        {{ initial }}
      </span>
      <span class="hidden min-w-0 truncate text-sm font-medium leading-tight md:inline">{{ auth.displayName }}</span>
      <Icon name="chevronDown" :size="14" class="hidden shrink-0 text-faint md:block" />
    </button>
    <Teleport to="body">
      <div
        v-if="open"
        ref="menu"
        role="menu"
        :aria-label="t('layout.accountMenu')"
        :style="style"
        class="overflow-hidden rounded-lg border border-line bg-surface py-1 shadow-lift"
      >
        <div class="border-b border-line px-3 py-2">
          <p class="truncate text-sm font-medium text-ink">{{ auth.displayName }}</p>
          <p class="text-[11px] text-faint">{{ roleLabel }}</p>
        </div>
        <button
          type="button"
          data-item
          role="menuitem"
          class="flex w-full items-center gap-2 px-3 py-2 text-left text-sm text-ink hover:bg-canvas"
          @click="goAccount"
        >
          <Icon name="user" :size="14" />
          {{ t('layout.viewAccount') }}
        </button>
        <button
          v-if="!auth.isAdmin"
          type="button"
          data-item
          role="menuitem"
          class="flex w-full items-center gap-2 px-3 py-2 text-left text-sm text-ink hover:bg-canvas"
          @click="goLoginLogs"
        >
          <Icon name="log" :size="14" />
          {{ t('layout.myLoginLogs') }}
        </button>
        <div class="my-1 border-t border-line" role="separator" />
        <button
          type="button"
          data-item
          role="menuitem"
          :class="cn('flex w-full items-center gap-2 px-3 py-2 text-left text-sm text-[var(--danger)] hover:bg-[rgba(214,69,69,.08)]')"
          @click="goLogout"
        >
          <Icon name="logout" :size="14" />
          {{ t('layout.logout') }}
        </button>
      </div>
    </Teleport>
  </div>
</template>
