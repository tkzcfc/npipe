import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'

const src = (...parts: string[]) =>
  readFileSync(resolve(__dirname, '../src', ...parts), 'utf8')

describe('shared overlays use the counted scroll lock', () => {
  const dialog = src('components/ui/Dialog.vue')
  const drawer = src('components/ui/Drawer.vue')
  const confirm = src('components/ui/ConfirmDialog.vue')
  const diagnose = src('components/DiagnoseDialog.vue')
  const editor = src('components/TunnelEditorDrawer.vue')
  const readonlyDrawer = src('components/TunnelReadonlyDrawer.vue')

  it('locks scroll through useScrollLock instead of assigning body overflow', () => {
    for (const [name, source] of [
      ['Dialog', dialog],
      ['Drawer', drawer],
    ] as const) {
      expect(source, name).toContain("import { useScrollLock } from '@/composables/useScrollLock'")
      expect(source, name).toContain('useScrollLock(isOpen)')
      expect(source, name).not.toMatch(/document\.body\.style\.overflow/)
      expect(source, name).not.toMatch(/documentElement\.style\.overflow/)
      expect(source, name).not.toMatch(/style\.width\s*=/)
    }
  })

  it('routes every shared dialog and drawer through Dialog or Drawer', () => {
    expect(confirm).toContain("import Dialog from './Dialog.vue'")
    expect(confirm).toMatch(/<Dialog[\s>]/)
    expect(confirm).not.toMatch(/document\.body\.style\.overflow/)

    expect(diagnose).toContain("import Dialog from '@/components/ui/Dialog.vue'")
    expect(diagnose).toMatch(/<Dialog[\s\S]*:open="/)

    expect(editor).toContain("import Drawer from '@/components/ui/Drawer.vue'")
    expect(editor).toMatch(/<Drawer[\s\S]*:open="/)

    expect(readonlyDrawer).toContain("import Drawer from '@/components/ui/Drawer.vue'")
    expect(readonlyDrawer).toMatch(/<Drawer[\s\S]*:open="/)
  })
})

describe('v5 shell layout', () => {
  const layout = src('layouts/MainLayout.vue')
  const account = src('components/AccountMenu.vue')

  it('keeps account in the header and pins a compact hamburger collapse control to the sidebar footer', () => {
    expect(layout).toContain("import AccountMenu from '@/components/AccountMenu.vue'")
    expect(layout).toContain("import { useScrollLock } from '@/composables/useScrollLock'")
    expect(layout).toContain('useScrollLock(mobileOpen)')
    expect(layout).toContain('appStore.toggleSidebar()')
    expect(layout).toContain('name="menu"')
    expect(layout).toContain(":title=\"appStore.sidebarCollapsed ? t('layout.expand') : t('layout.collapse')\"")
    expect(layout).toContain(":aria-label=\"appStore.sidebarCollapsed ? t('layout.expand') : t('layout.collapse')\"")
    expect(layout).toContain(':aria-expanded="!appStore.sidebarCollapsed"')
    expect(layout).toContain('mt-auto hidden shrink-0 overflow-visible border-t border-line p-2 md:block')
    expect(layout).toContain('h-10 w-10')
    expect(layout).toContain('role="tooltip"')
    expect(layout).toContain('group-hover:block')
    expect(layout).toContain('group-focus-within:block')
    expect(layout).not.toContain("mobileOpen && 'max-md:overflow-hidden'")
    expect(layout).not.toMatch(/chevronRight|chevronLeft/)
    expect(layout).not.toMatch(/<span[^>]*>\{\{\s*t\('layout\.collapse'\)\s*\}\}<\/span>/)
    const aside = layout.slice(layout.indexOf('<aside'), layout.indexOf('</aside>'))
    expect(aside).not.toContain('AccountMenu')
    expect(aside).not.toContain('displayName')
    expect(aside.indexOf('name="menu"')).toBeGreaterThan(aside.indexOf('</nav>'))
    const footer = aside.slice(aside.indexOf('mt-auto'))
    expect(footer).toContain('h-10 w-10')
    expect(footer).not.toContain('w-full')
    const header = layout.slice(layout.indexOf('<header'), layout.indexOf('</header>'))
    expect(header).toContain('<AccountMenu')
    expect(header).toContain('<LanguageSwitch compact />')
    expect(header).toContain('<ThemeSwitch />')
  })

  it('positions the account menu with the top-right placement utility', () => {
    expect(account).toContain("import { placeMenu } from '@/utils/menuPlacement'")
    expect(account).toContain("align: 'end'")
    expect(account).toContain('roleLabel')
    expect(account).toContain('auth.displayName')
    expect(account).toContain("t('layout.viewAccount')")
    expect(account).toContain("t('layout.myLoginLogs')")
    expect(account).toContain("t('layout.logout')")
    expect(account).toContain('md:inline')
    expect(account).toContain("emit('account')")
    expect(account).not.toMatch(/height === 176|rect\.top - 176|rect\.top - 132/)
  })
})
