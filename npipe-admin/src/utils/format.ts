import { parseServerTime } from './time'

const pad = (n: number) => n.toString().padStart(2, '0')

export function formatBytes(n: number | null | undefined): string {
  if (n === null || n === undefined || Number.isNaN(n)) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let value = Math.abs(n)
  let unit = 0
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024
    unit += 1
  }
  const digits = unit === 0 ? 0 : unit === 1 ? 1 : 2
  return `${value.toFixed(digits)} ${units[unit]}`
}

export function formatDuration(totalSeconds: number | null | undefined): string {
  const secs = Math.max(0, Math.floor(Number(totalSeconds) || 0))
  const days = Math.floor(secs / 86400)
  const hours = Math.floor((secs % 86400) / 3600)
  const minutes = Math.floor((secs % 3600) / 60)
  const seconds = secs % 60
  if (days > 0) return `${days}d ${hours}h`
  if (hours > 0) return `${hours}h ${minutes}m`
  if (minutes > 0) return `${minutes}m ${seconds}s`
  return `${seconds}s`
}

export function formatPercent(value: number): string {
  const n = Math.max(0, Math.min(100, Number(value) || 0))
  return `${n.toFixed(1)}%`
}

export function formatHourKey(date: Date): string {
  return `${date.getUTCFullYear()}-${pad(date.getUTCMonth() + 1)}-${pad(date.getUTCDate())} ${pad(date.getUTCHours())}`
}

export function formatHourLabel(hour: string, compact: boolean): string {
  const d = parseServerTime(hour) ?? new Date(`${hour}:00:00Z`)
  if (Number.isNaN(d.getTime())) return hour
  if (compact) return `${pad(d.getHours())}:00`
  return `${pad(d.getMonth() + 1)}/${pad(d.getDate())} ${pad(d.getHours())}:00`
}
