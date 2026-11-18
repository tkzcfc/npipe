<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { cn } from '@/utils/cn'
import { placeMenu } from '@/utils/menuPlacement'
import Button from './Button.vue'
import Icon from './Icon.vue'

export interface ActionMenuItem {
  key: string
  label: string
  icon?: string
  tone?: 'default' | 'danger'
  disabled?: boolean
  hidden?: boolean
}

const props = withDefaults(defineProps<{
  items: ActionMenuItem[]
  label?: string
}>(), {
  label: '',
})

const emit = defineEmits<{
  select: [key: string]
}>()

const { t } = useI18n()
const open = ref(false)
const trigger = ref<HTMLElement | null>(null)
const menu = ref<HTMLElement | null>(null)
const style = ref<Record<string, string>>({})

const visibleItems = () => props.items.filter((item) => !item.hidden)

function place() {
  const rect = trigger.value?.getBoundingClientRect()
  if (!rect) return
  const pos = placeMenu({
    trigger: { top: rect.top, right: rect.right, bottom: rect.bottom, left: rect.left },
    menuWidth: 200,
    menuHeight: visibleItems().length * 36 + 8,
    viewportWidth: window.innerWidth,
    viewportHeight: window.innerHeight,
    align: 'end',
  })
  style.value = {
    position: 'fixed',
    top: `${pos.top}px`,
    left: `${pos.left}px`,
    width: `${pos.width}px`,
    zIndex: '90',
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

function choose(item: ActionMenuItem) {
  if (item.disabled) return
  emit('select', item.key)
  close()
}
</script>

<template>
  <div ref="trigger" class="inline-flex">
    <Button
      variant="ghost"
      size="sm"
      icon="more"
      :aria-label="label || t('common.more')"
      :aria-haspopup="true"
      :aria-expanded="open"
      @click="toggle"
    />
    <Teleport to="body">
      <div
        v-if="open"
        ref="menu"
        role="menu"
        :style="style"
        class="rounded-lg border border-line bg-surface py-1 shadow-lift"
      >
        <button
          v-for="item in visibleItems()"
          :key="item.key"
          type="button"
          data-item
          role="menuitem"
          :disabled="item.disabled"
          :class="cn(
            'flex w-full items-center gap-2 px-3 py-2 text-left text-sm disabled:opacity-50',
            item.tone === 'danger' ? 'text-[var(--danger)] hover:bg-[rgba(214,69,69,.08)]' : 'text-ink hover:bg-canvas',
          )"
          @click="choose(item)"
        >
          <Icon v-if="item.icon" :name="item.icon" :size="14" />
          <span>{{ item.label }}</span>
        </button>
      </div>
    </Teleport>
  </div>
</template>
