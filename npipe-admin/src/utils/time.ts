/**
 * 统一的时间格式化工具：所有时间都按浏览器本地时区显示。
 *
 * 支持的输入：
 * - number：Unix 时间戳（秒；>1e12 视为毫秒）
 * - RFC3339 / ISO 字符串（带 Z 或偏移）
 * - 旧格式 "YYYY-MM-DD HH:mm:ss" / "YYYY-MM-DD HH"（服务端按 UTC 存储，无时区标记，按 UTC 解析）
 */
export function parseServerTime(value: number | string | null | undefined): Date | null {
  if (value === null || value === undefined || value === '' || value === 0) return null
  if (typeof value === 'number') {
    return new Date(value > 1e12 ? value : value * 1000)
  }
  let s = value.trim()
  if (/^\d+$/.test(s)) return parseServerTime(Number(s))
  if (/^\d{4}-\d{2}-\d{2} \d{2}$/.test(s)) s = s.replace(' ', 'T') + ':00:00Z'
  else if (/^\d{4}-\d{2}-\d{2}[ T]\d{2}:\d{2}(:\d{2}(\.\d+)?)?$/.test(s)) s = s.replace(' ', 'T') + 'Z'
  const d = new Date(s)
  return isNaN(d.getTime()) ? null : d
}

const pad = (n: number) => n.toString().padStart(2, '0')

export function formatDateTime(value: number | string | null | undefined, fallback = '-'): string {
  const d = parseServerTime(value)
  if (!d) return fallback
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
}

export function formatHour(value: number | string | null | undefined, fallback = '-'): string {
  const d = parseServerTime(value)
  if (!d) return fallback
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:00`
}
