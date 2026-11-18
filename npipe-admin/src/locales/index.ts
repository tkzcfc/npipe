import { createI18n } from 'vue-i18n'
import zhCN from './zh-CN'
import enUS from './en-US'
import { htmlLang, readLocale, type LocaleCode, LOCALE_STORAGE_KEY } from '@/utils/locale'

type MessageSchema = typeof zhCN

const initialLocale = typeof localStorage === 'undefined' ? 'zh-CN' : readLocale(localStorage)

const i18n = createI18n<[MessageSchema], 'zh-CN' | 'en-US'>({
  legacy: false,
  locale: initialLocale,
  fallbackLocale: 'zh-CN',
  messages: {
    'zh-CN': zhCN,
    'en-US': enUS,
  },
  silentTranslationWarn: true,
  missingWarn: false,
  fallbackWarn: false,
})

export default i18n

export function setLanguage(lang: LocaleCode) {
  ;(i18n.global.locale as unknown as { value: LocaleCode }).value = lang
  localStorage.setItem(LOCALE_STORAGE_KEY, lang)
  document.documentElement.setAttribute('lang', htmlLang(lang))
}

export function currentLocale(): LocaleCode {
  return (i18n.global.locale as unknown as { value: LocaleCode }).value
}
