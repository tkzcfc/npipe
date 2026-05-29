import request from './request'
import type { GeneralResponse, LoginResponse as LoginResponseType } from '@/types'

export interface LoginRequest {
  username: string
  password: string
}

export const authApi = {
  login(data: LoginRequest) {
    return request.post<LoginResponseType>('/api/login', data)
  },
  logout() {
    return request.post<GeneralResponse>('/api/logout', {})
  },
  testAuth() {
    return request.post<LoginResponseType>('/api/test_auth', {})
  },
}

