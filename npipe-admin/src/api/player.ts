import request from './request'
import type {
  GeneralResponse,
  PlayerListRequest,
  PlayerListResponse,
  PlayerAddRequest,
  PlayerUpdateRequest,
  PlayerRemoveRequest,
} from '@/types'

export const playerApi = {
  list(data: PlayerListRequest) {
    return request.post<PlayerListResponse>('/api/player_list', data)
  },
  add(data: PlayerAddRequest) {
    return request.post<GeneralResponse>('/api/add_player', data)
  },
  update(data: PlayerUpdateRequest) {
    return request.post<GeneralResponse>('/api/update_player', data)
  },
  remove(data: PlayerRemoveRequest) {
    return request.post<GeneralResponse>('/api/remove_player', data)
  },
}

