import axios, { type AxiosInstance, type AxiosResponse, type InternalAxiosRequestConfig } from 'axios'
import { ElMessage } from 'element-plus'
import router from '@/router'
import { useAuthStore } from '@/stores/auth'

const request: AxiosInstance = axios.create({
  baseURL: '/',
  timeout: 8000,
  withCredentials: true,
  headers: {
    'Content-Type': 'application/json',
  },
})

// Request interceptor
request.interceptors.request.use(
  (config: InternalAxiosRequestConfig) => {
    return config
  },
  (error) => Promise.reject(error),
)

// Response interceptor
function handleSessionExpired() {
  const authStore = useAuthStore()
  authStore.clearSession()
  if (router.currentRoute.value.name !== 'Login') {
    ElMessage.error('登录已过期，请重新登录')
    router.replace({ name: 'Login', query: { redirect: router.currentRoute.value.fullPath } })
  }
}

request.interceptors.response.use(
  (response: AxiosResponse) => {
    const data = response.data
    // Session expired
    if (data?.code === 10086) {
      handleSessionExpired()
      return Promise.reject(new Error('Session expired'))
    }
    return response
  },
  (error) => {
    const status = error.response?.status
    if (error.code === 'ECONNABORTED') {
      ElMessage.error('请求超时，请检查网络后重试')
      return Promise.reject(error)
    }
    if (!error.response) {
      ElMessage.error('网络连接异常，请稍后重试')
      return Promise.reject(error)
    }
    if (status === 401) {
      handleSessionExpired()
      return Promise.reject(error)
    }
    const messages: Record<number, string> = {
      400: '请求参数错误',
      401: '未授权，请重新登录',
      403: '拒绝访问',
      404: '请求路径不存在',
      500: '服务器内部错误',
      502: '网关错误',
      503: '服务不可用',
    }
    ElMessage.error(messages[status] ?? `网络错误 (${status ?? 'unknown'})`)
    return Promise.reject(error)
  },
)

export default request

