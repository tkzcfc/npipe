import request from './request'
import type {
  GeneralResponse,
  TunnelListRequest,
  TunnelListResponse,
  TunnelMutateRequest,
  TunnelRemoveRequest,
} from '@/types'

export const tunnelApi = {
  list(data: TunnelListRequest) {
    return request.post<TunnelListResponse>('/api/tunnel_list', data)
  },
  add(data: TunnelMutateRequest) {
    return request.post<GeneralResponse>('/api/add_tunnel', data)
  },
  update(data: TunnelMutateRequest) {
    return request.post<GeneralResponse>('/api/update_tunnel', data)
  },
  remove(data: TunnelRemoveRequest) {
    return request.post<GeneralResponse>('/api/remove_tunnel', data)
  },
}

