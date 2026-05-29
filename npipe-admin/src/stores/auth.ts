import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { authApi } from '@/api'

export const useAuthStore = defineStore('auth', () => {
  const isLoggedIn = ref(false)
  const role = ref<string>('')        // 'admin' | 'user' | ''
  const currentUserId = ref<number>(0)

  const isAdmin = computed(() => role.value === 'admin')
  const isUser = computed(() => role.value === 'user')

  async function checkAuth(): Promise<boolean> {
    try {
      const res = await authApi.testAuth()
      isLoggedIn.value = res.data.code === 0
      role.value = res.data.role ?? ''
      currentUserId.value = res.data.user_id ?? 0
    } catch {
      isLoggedIn.value = false
      role.value = ''
      currentUserId.value = 0
    }
    return isLoggedIn.value
  }

  async function login(username: string, password: string): Promise<{ ok: boolean; msg: string }> {
    const res = await authApi.login({ username, password })
    if (res.data.code === 0) {
      isLoggedIn.value = true
      role.value = res.data.role ?? ''
      currentUserId.value = res.data.user_id ?? 0
      return { ok: true, msg: res.data.msg }
    }
    return { ok: false, msg: res.data.msg }
  }

  async function logout() {
    try { await authApi.logout() } catch { /* ignore */ }
    clearSession()
  }

  function clearSession() {
    isLoggedIn.value = false
    role.value = ''
    currentUserId.value = 0
  }

  return { isLoggedIn, role, currentUserId, isAdmin, isUser, checkAuth, login, logout, clearSession }
})

