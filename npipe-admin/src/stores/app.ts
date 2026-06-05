import { defineStore } from 'pinia'
import { ref } from 'vue'

export type Theme = 'dark' | 'light'

export const useAppStore = defineStore('app', () => {
  const theme = ref<Theme>((localStorage.getItem('theme') as Theme) ?? 'dark')
  const sidebarCollapsed = ref(false)
  const routeLoading = ref(false)

  function setTheme(t: Theme) {
    theme.value = t
    localStorage.setItem('theme', t)
    document.documentElement.setAttribute('data-theme', t)
    if (t === 'dark') {
      document.documentElement.classList.add('dark')
    } else {
      document.documentElement.classList.remove('dark')
    }
  }

  function toggleTheme() {
    setTheme(theme.value === 'dark' ? 'light' : 'dark')
  }

  function toggleSidebar() {
    sidebarCollapsed.value = !sidebarCollapsed.value
  }

  function setRouteLoading(loading: boolean) {
    routeLoading.value = loading
  }

  // init on creation
  setTheme(theme.value)

  return { theme, sidebarCollapsed, routeLoading, setTheme, toggleTheme, toggleSidebar, setRouteLoading }
})
