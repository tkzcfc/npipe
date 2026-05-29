import request from './request'
import type {
  GeneralResponse,
  TunnelDetailRequest,
  TunnelDetailResponse,
  TunnelDiagnoseRequest,
  TunnelDiagnoseResponse,
  TunnelListRequest,
  TunnelListResponse,
  TunnelMutateRequest,
  TunnelRemoveRequest,
  TunnelStatusUpdateRequest,
} from '@/types'

export const tunnelApi = {
  list(data: TunnelListRequest) {
    return request.post<TunnelListResponse>('/api/tunnel_list', data)
  },
  detail(data: TunnelDetailRequest) {
    return request.post<TunnelDetailResponse>('/api/tunnel_detail', data)
  },
  add(data: TunnelMutateRequest) {
    return request.post<GeneralResponse>('/api/add_tunnel', data)
  },
  update(data: TunnelMutateRequest) {
    return request.post<GeneralResponse>('/api/update_tunnel', data)
  },
  updateStatus(data: TunnelStatusUpdateRequest) {
    return request.post<GeneralResponse>('/api/update_tunnel_status', data)
  },
  diagnose(data: TunnelDiagnoseRequest) {
    return request.post<TunnelDiagnoseResponse>('/api/tunnel_diagnose', data)
  },
  remove(data: TunnelRemoveRequest) {
    return request.post<GeneralResponse>('/api/remove_tunnel', data)
  },
}

