<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAuthStore } from '@/stores/auth'
import { cn } from '@/utils/cn'
import Icon from '@/components/ui/Icon.vue'

const props = defineProps<{
  collapsed?: boolean
}>()

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
  const width = Math.max(200, props.collapsed ? 220 : rect.width)
  let left = props.collapsed ? rect.right + 8 : rect.left
  left = Math.min(Math.max(8, left), window.innerWidth - width - 8)
  const height = auth.isAdmin ? 132 : 176
  let top = rect.top - height - 8
  if (top < 8) top = Math.min(window.innerHeight - height - 8, rect.bottom + 8)
  style.value = {
    position: 'fixed',
    top: `${top}px`,
    left: `${left}px`,
    width: `${width}px`,
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

watch(open, async (value) => {
  if (value) {
    place()
    await nextTick()
    place()
    menu.value?.querySelector<HTMLButtonElement>('button[data-item]')?.focus()
    document.addEventListener('mousedown', onDoc)
    document.addEventListener('keydown', onKey)
    window.addEventListener('resize', close)
    window.addEventListener('scroll', close, true)
  } else {
    document.removeEventListener('mousedown', onDoc)
    document.removeEventListener('keydown', onKey)
    window.removeEventListener('resize', close)
    window.removeEventListener('scroll', close, true)
  }
})

onBeforeUnmount(close)

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
  <div ref="trigger" class="px-2 pb-2 pt-1">
    <button
      type="button"
      class="flex w-full items-center gap-2 rounded-lg px-2 py-2 text-left text-[var(--text-sidebar)] hover:bg-[var(--bg-sidebar-hover)]"
      :class="collapsed && 'justify-center px-0'"
      :aria-haspopup="true"
      :aria-expanded="open"
      :aria-label="t('layout.accountMenu')"
      :title="auth.displayName"
      @click="toggle"
    >
      <span class="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-accent-dim text-xs font-semibold text-accent">
        {{ initial }}
      </span>
      <span v-if="!collapsed" class="min-w-0 flex-1">
        <span class="block truncate text-sm font-medium leading-tight">{{ auth.displayName }}</span>
        <span class="block truncate text-[11px] text-[var(--text-sidebar-muted)]">{{ roleLabel }}</span>
      </span>
      <Icon v-if="!collapsed" name="chevronUp" :size="14" class="shrink-0 text-[var(--text-sidebar-muted)]" />
    </button>
    <Teleport to="body">
      <div
        v-if="open"
        ref="menu"
        role="menu"
        :style="style"
        class="rounded-lg border border-line bg-surface py-1 shadow-lift"
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
