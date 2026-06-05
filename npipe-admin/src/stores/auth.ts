import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { authApi } from '@/api'

export const useAuthStore = defineStore('auth', () => {
  const isLoggedIn = ref(false)
  const role = ref<string>('')        // 'admin' | 'user' | ''
  const currentUserId = ref<number>(0)
  const username = ref<string>('')
  let checkAuthPromise: Promise<boolean> | null = null

  const isAdmin = computed(() => role.value === 'admin')
  const isUser = computed(() => role.value === 'user')
  const displayName = computed(() => username.value || (isAdmin.value ? 'Admin' : currentUserId.value ? `User #${currentUserId.value}` : 'User'))

  async function checkAuth(): Promise<boolean> {
    if (checkAuthPromise) return checkAuthPromise

    checkAuthPromise = (async () => {
      try {
        const res = await authApi.testAuth()
        isLoggedIn.value = res.data.code === 0
        role.value = res.data.role ?? ''
        currentUserId.value = res.data.user_id ?? 0
        username.value = res.data.username ?? ''
      } catch {
        isLoggedIn.value = false
        role.value = ''
        currentUserId.value = 0
        username.value = ''
      } finally {
        checkAuthPromise = null
      }
      return isLoggedIn.value
    })()

    return checkAuthPromise
  }

  async function login(loginUsername: string, password: string): Promise<{ ok: boolean; msg: string }> {
    const res = await authApi.login({ username: loginUsername, password })
    if (res.data.code === 0) {
      isLoggedIn.value = true
      role.value = res.data.role ?? ''
      currentUserId.value = res.data.user_id ?? 0
      username.value = res.data.username ?? loginUsername
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
    username.value = ''
  }

  return { isLoggedIn, role, currentUserId, username, displayName, isAdmin, isUser, checkAuth, login, logout, clearSession }
})

