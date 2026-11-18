import axios, { type AxiosInstance, type AxiosResponse, type InternalAxiosRequestConfig } from 'axios'
import router from '@/router'
import { useAuthStore } from '@/stores/auth'
import { useToastStore } from '@/stores/toast'
import i18n from '@/locales'

const request: AxiosInstance = axios.create({
  baseURL: '/',
  timeout: 8000,
  withCredentials: true,
  headers: {
    'Content-Type': 'application/json',
  },
})

function t(key: string) {
  return String(i18n.global.t(key))
}

function handleSessionExpired() {
  const authStore = useAuthStore()
  authStore.clearSession()
  if (router.currentRoute.value.name !== 'Login') {
    useToastStore().error(t('error.sessionExpired'))
    router.replace({ name: 'Login', query: { redirect: router.currentRoute.value.fullPath } })
  }
}

request.interceptors.request.use(
  (config: InternalAxiosRequestConfig) => config,
  (error) => Promise.reject(error),
)

request.interceptors.response.use(
  (response: AxiosResponse) => {
    const data = response.data
    if (data?.code === 10086) {
      handleSessionExpired()
      return Promise.reject(new Error('Session expired'))
    }
    return response
  },
  (error) => {
    const status = error.response?.status
    if (error.code === 'ECONNABORTED') {
      useToastStore().error(t('common.timeout'))
      return Promise.reject(error)
    }
    if (!error.response) {
      useToastStore().error(t('common.networkError'))
      return Promise.reject(error)
    }
    if (status === 401) {
      handleSessionExpired()
      return Promise.reject(error)
    }
    const messages: Record<number, string> = {
      400: t('common.badRequest'),
      403: t('common.forbidden'),
      404: t('common.notFound'),
      429: error.response?.data?.msg || t('common.failed'),
      500: t('common.serverError'),
      502: t('common.gatewayError'),
      503: t('common.unavailable'),
    }
    if (status !== 429) {
      useToastStore().error(messages[status] ?? `${t('common.unknownError')} (${status ?? 'unknown'})`)
    }
    return Promise.reject(error)
  },
)

export default request
