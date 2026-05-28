import axios, { type AxiosInstance, type AxiosResponse, type InternalAxiosRequestConfig } from 'axios'
import { ElMessage } from 'element-plus'
import router from '@/router'

const request: AxiosInstance = axios.create({
  baseURL: '/',
  timeout: 15000,
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
request.interceptors.response.use(
  (response: AxiosResponse) => {
    const data = response.data
    // Session expired
    if (data?.code === 10086) {
      ElMessage.error('登录已过期，请重新登录')
      router.push('/login')
      return Promise.reject(new Error('Session expired'))
    }
    return response
  },
  (error) => {
    const status = error.response?.status
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

