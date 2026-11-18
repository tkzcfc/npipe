export type ApiResultLike = {
  code?: number
  msg?: string
} | null | undefined

export function messageFromResult(
  result: ApiResultLike,
  translate: (key: string) => string,
  fallback: string,
): string {
  if (result?.code === 403) return translate('common.forbidden')
  if (typeof result?.msg === 'string' && result.msg.trim() && result.msg !== 'Forbidden') {
    return result.msg
  }
  if (result?.msg === 'Forbidden') return translate('common.forbidden')
  return fallback
}

export function messageFromUnknown(
  error: unknown,
  translate: (key: string) => string,
  fallback: string,
): string {
  const responseData = typeof error === 'object' && error !== null && 'response' in error
    ? (error as { response?: { data?: ApiResultLike; status?: number } }).response
    : undefined
  if (responseData?.status === 403 || responseData?.data?.code === 403) {
    return translate('common.forbidden')
  }
  return messageFromResult(responseData?.data, translate, fallback)
}
