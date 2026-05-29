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
}

// ── Dashboard ──────────────────────────────────────────────────────────────
export interface DashboardConfigInfo {
  listen_addr: string
  web_addr: string
  enable_tls: boolean
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

export interface PlayerRemoveRequest {
  id: number
}

export interface KickPlayerRequest {
  id: number
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
