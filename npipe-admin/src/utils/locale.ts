export type LocaleCode = 'zh-CN' | 'en-US'

export const LOCALE_STORAGE_KEY = 'npipe-lang'
export const LOCALE_LEGACY_KEY = 'lang'
export const DEFAULT_LOCALE: LocaleCode = 'zh-CN'

export function isLocaleCode(value: unknown): value is LocaleCode {
  return value === 'zh-CN' || value === 'en-US'
}

export function readLocale(
  storage: Pick<Storage, 'getItem'>,
  keys: string[] = [LOCALE_STORAGE_KEY, LOCALE_LEGACY_KEY],
  fallback: LocaleCode = DEFAULT_LOCALE,
): LocaleCode {
  for (const key of keys) {
    const value = storage.getItem(key)
    if (isLocaleCode(value)) return value
  }
  return fallback
}

export function htmlLang(locale: LocaleCode): string {
  return locale === 'en-US' ? 'en' : 'zh-CN'
}
