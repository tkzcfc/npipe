import request from './request'
import type {
  GeneralResponse,
  PlayerListRequest,
  PlayerListResponse,
  PlayerAddRequest,
  PlayerUpdateRequest,
  PlayerRenameRequest,
  PlayerResetPasswordRequest,
  PlayerStatusUpdateRequest,
  PlayerWebAccessUpdateRequest,
  PlayerRemoveRequest,
  KickPlayerRequest,
  PlayerDetailRequest,
  PlayerDetailResponse,
  TrafficStatsRequest,
  TrafficStatsResponse,
  LoginHistoryRequest,
  LoginHistoryResponse,
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
  rename(data: PlayerRenameRequest) {
    return request.post<GeneralResponse>('/api/rename_player', data)
  },
  resetPassword(data: PlayerResetPasswordRequest) {
    return request.post<GeneralResponse>('/api/reset_player_password', data)
  },
  updateStatus(data: PlayerStatusUpdateRequest) {
    return request.post<GeneralResponse>('/api/update_player_status', data)
  },
  updateWebAccess(data: PlayerWebAccessUpdateRequest) {
    return request.post<GeneralResponse>('/api/update_player_web_access', data)
  },
  remove(data: PlayerRemoveRequest) {
    return request.post<GeneralResponse>('/api/remove_player', data)
  },
  kick(data: KickPlayerRequest) {
    return request.post<GeneralResponse>('/api/kick_player', data)
  },
  detail(data: PlayerDetailRequest) {
    return request.post<PlayerDetailResponse>('/api/player_detail', data)
  },
  trafficStats(data: TrafficStatsRequest) {
    return request.post<TrafficStatsResponse>('/api/traffic_stats', data)
  },
  loginHistory(data: LoginHistoryRequest) {
    return request.post<LoginHistoryResponse>('/api/login_history', data)
  },
}

