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

// ── Player ─────────────────────────────────────────────────────────────────
export interface Player {
  id: number
  username: string
  password: string
  online: boolean
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

export interface PlayerRemoveRequest {
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
  password: string
  username: string
  is_compressed: boolean
  encryption_method: EncryptionMethod
  custom_mapping: Record<string, string>
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
}

export interface TunnelRemoveRequest {
  id: number
}

