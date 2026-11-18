import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { setLanguage, currentLocale } from '@/locales'
import {
  THEME_STORAGE_KEY,
  applyResolvedTheme,
  readThemeMode,
  resolveTheme,
  type ResolvedTheme,
  type ThemeMode,
} from '@/utils/theme'
import type { LocaleCode } from '@/utils/locale'

function prefersDark() {
  return window.matchMedia('(prefers-color-scheme: dark)').matches
}

export const useAppStore = defineStore('app', () => {
  const themeMode = ref<ThemeMode>(typeof localStorage === 'undefined' ? 'system' : readThemeMode(localStorage))
  const resolvedTheme = ref<ResolvedTheme>(resolveTheme(themeMode.value, typeof window === 'undefined' ? true : prefersDark()))
  const locale = ref<LocaleCode>(currentLocale())
  const sidebarCollapsed = ref(false)
  const routeLoading = ref(false)
  let media: MediaQueryList | null = null

  function syncResolved() {
    resolvedTheme.value = resolveTheme(themeMode.value, prefersDark())
    applyResolvedTheme(resolvedTheme.value, themeMode.value)
  }

  function setThemeMode(mode: ThemeMode) {
    themeMode.value = mode
    localStorage.setItem(THEME_STORAGE_KEY, mode)
    syncResolved()
  }

  function setLocale(next: LocaleCode) {
    setLanguage(next)
    locale.value = next
  }

  function toggleSidebar() {
    sidebarCollapsed.value = !sidebarCollapsed.value
  }

  function setRouteLoading(loading: boolean) {
    routeLoading.value = loading
  }

  function initThemeListener() {
    syncResolved()
    media = window.matchMedia('(prefers-color-scheme: dark)')
    const onChange = () => {
      if (themeMode.value === 'system') syncResolved()
    }
    media.addEventListener('change', onChange)
    return () => media?.removeEventListener('change', onChange)
  }

  const isDark = computed(() => resolvedTheme.value === 'dark')

  return {
    themeMode,
    resolvedTheme,
    isDark,
    locale,
    sidebarCollapsed,
    routeLoading,
    setThemeMode,
    setLocale,
    toggleSidebar,
    setRouteLoading,
    initThemeListener,
  }
})
