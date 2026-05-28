import { defineStore } from 'pinia'
import { ref } from 'vue'
import { authApi } from '@/api'

export const useAuthStore = defineStore('auth', () => {
  const isLoggedIn = ref(false)

  async function checkAuth(): Promise<boolean> {
    try {
      const res = await authApi.testAuth()
      isLoggedIn.value = res.data.code === 0
    } catch {
      isLoggedIn.value = false
    }
    return isLoggedIn.value
  }

  async function login(username: string, password: string): Promise<{ ok: boolean; msg: string }> {
    const res = await authApi.login({ username, password })
    if (res.data.code === 0) {
      isLoggedIn.value = true
      return { ok: true, msg: res.data.msg }
    }
    return { ok: false, msg: res.data.msg }
  }

  async function logout() {
    try { await authApi.logout() } catch { /* ignore */ }
    isLoggedIn.value = false
  }

  return { isLoggedIn, checkAuth, login, logout }
})

