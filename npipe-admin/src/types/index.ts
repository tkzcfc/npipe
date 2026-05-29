// ── General ────────────────────────────────────────────────────────────────
export interface ApiResponse<T = unknown> {
  code: number
  msg: string
  data?: T
}

export interface GeneralResponse {
  code: number
  msg: string
}

export interface LoginResponse {
  code: number
  msg: string
  role: string | null
  user_id?: number | null
  username?: string | null
}

// ── Dashboard ──────────────────────────────────────────────────────────────
export interface DashboardConfigInfo {
  listen_addr: string
  web_addr: string
  enable_tls: boolean
  web_enable_tls: boolean
  web_tls_cert: string
  web_tls_auto_self_signed: boolean
  web_cookie_secure: boolean
  tls_cert: string
  web_base_dir: string
  illegal_traffic_forward: string
  quiet: boolean
  log_dir: string
  database: string
}

export interface DashboardSystemInfo {
  host_name: string
  os_name: string
  kernel_version: string
  uptime_secs: number
  cpu_usage: number
  cpu_cores: number
  total_memory: number
  used_memory: number
  memory_usage: number
}

export interface DashboardOverviewResponse {
  online_players: number
  total_players: number
  enabled_tunnels: number
  total_tunnels: number
  config: DashboardConfigInfo
  system: DashboardSystemInfo
}

// ── Player ─────────────────────────────────────────────────────────────────
export interface Player {
  id: number
  username: string
  enabled: boolean
  web_access: boolean
  online: boolean
  ip_addr: string
  online_time: number
  bytes_in: number
  bytes_out: number
}

export interface TrafficStatsRequest {
  user_id: number
  hours?: number
}

export interface TrafficHourItem {
  hour: string
  bytes_in: number
  bytes_out: number
}

export interface TrafficStatsResponse {
  items: TrafficHourItem[]
  total_in: number
  total_out: number
}

// ── Login History ──────────────────────────────────────────────────────────
export interface LoginHistoryRequest {
  user_id?: number
  page_number?: number
  page_size?: number
}

export interface LoginHistoryItem {
  id: number
  user_id: number
  ip_addr: string
  login_time: string
  logout_time: string
  duration_secs: number
}

export interface LoginHistoryResponse {
  items: LoginHistoryItem[]
  total_count: number
}

export interface OperationLogRequest {
  page_number?: number
  page_size?: number
}

export interface OperationLogItem {
  id: number
  actor: string
  action: string
  target_type: string
  target_id: number
  target_name: string
  detail: string
  created_at: string
}

export interface OperationLogResponse {
  items: OperationLogItem[]
  total_count: number
}

export interface CleanupDatabaseRequest {
  login_history_keep_days?: number
  operation_log_keep_days?: number
  traffic_hourly_keep_days?: number
}

export interface CleanupDatabaseResponse {
  login_history_deleted: number
  operation_log_deleted: number
  traffic_hourly_deleted: number
}

export interface DatabaseMaintenanceTableInfo {
  total_count: number
  cleanup_count: number
  oldest: string
  newest: string
}

export interface DatabaseMaintenanceInfoResponse {
  login_history: DatabaseMaintenanceTableInfo
  operation_log: DatabaseMaintenanceTableInfo
  traffic_hourly: DatabaseMaintenanceTableInfo
}

export interface PlayerListRequest {
  page_number: number
  page_size: number
}

export interface PlayerListResponse {
  players: Player[]
  cur_page_number: number
  total_count: number
}

export interface PlayerAddRequest {
  username: string
  password: string
}

export interface PlayerUpdateRequest {
  id: number
  username: string
  password: string
}

export interface PlayerRenameRequest {
  id: number
  username: string
}

export interface PlayerResetPasswordRequest {
  id: number
  password: string
}

export interface PlayerStatusUpdateRequest {
  id: number
  enabled: number
}

export interface PlayerWebAccessUpdateRequest {
  id: number
  web_access: number
}

export interface PlayerRemoveRequest {
  id: number
}

export interface KickPlayerRequest {
  id: number
}

export interface PlayerDetailRequest {
  id: number
}

export interface PlayerTunnelItem {
  id: number
  source: string
  endpoint: string
  enabled: boolean
  tunnel_type: number
  role: string
  available: boolean
}

export interface PlayerDetail {
  id: number
  username: string
  enabled: boolean
  web_access: boolean
  create_time: string
  online: boolean
  ip_addr: string
  online_time: number
  bytes_in: number
  bytes_out: number
  traffic_24h_in: number
  traffic_24h_out: number
  tunnels: PlayerTunnelItem[]
  recent_logins: LoginHistoryItem[]
}

export interface PlayerDetailResponse {
  player: PlayerDetail | null
}

// ── Tunnel ─────────────────────────────────────────────────────────────────
export type TunnelType = 0 | 1 | 2 | 3 // 0=TCP 1=UDP 2=SOCKS5 3=HTTP
export type EncryptionMethod = 'None' | 'Xor' | 'Aes128'

export interface Tunnel {
  id: number
  source: string
  endpoint: string
  enabled: boolean
  sender: number
  receiver: number
  description: string
  tunnel_type: TunnelType
  username: string
  is_compressed: boolean
  encryption_method: EncryptionMethod
  custom_mapping: Record<string, string>
  sender_online: boolean
  receiver_online: boolean
  available: boolean
}

export interface TunnelDetail extends Tunnel {
  password: string
}

export interface TunnelListRequest {
  page_number: number
  page_size: number
}

export interface TunnelListResponse {
  tunnels: Tunnel[]
  cur_page_number: number
  total_count: number
}

export interface TunnelDetailRequest {
  id: number
}

export interface TunnelDetailResponse {
  tunnel: TunnelDetail | null
}

export interface TunnelMutateRequest {
  id?: number
  source: string
  endpoint: string
  enabled: number  // 0 | 1
  sender: number
  receiver: number
  description: string
  tunnel_type: number
  password: string
  username: string
  is_compressed: number  // 0 | 1
  encryption_method: string
  custom_mapping: Record<string, string>
  preserve_password?: boolean
}

export interface TunnelRemoveRequest {
  id: number
}

export interface TunnelStatusUpdateRequest {
  id: number
  enabled: number
}

export interface TunnelDiagnoseRequest {
  id?: number
  source: string
  endpoint: string
  sender: number
  receiver: number
  tunnel_type: number
}

export interface TunnelDiagnoseItem {
  key: string
  level: 'ok' | 'warn' | 'error'
  message: string
}

export interface TunnelDiagnoseResponse {
  ok: boolean
  items: TunnelDiagnoseItem[]
}
