import { createI18n } from 'vue-i18n'
import zhCN from './zh-CN'
import enUS from './en-US'

// 类型定义：确保 zh-CN 和 en-US 的 key 完全一致
type MessageSchema = typeof zhCN

// 从 localStorage 读取用户语言偏好，默认中文
const savedLang = localStorage.getItem('lang')
const defaultLocale = (savedLang === 'en-US' || savedLang === 'zh-CN') ? savedLang : 'zh-CN'

const i18n = createI18n<[MessageSchema], 'zh-CN' | 'en-US'>({
  legacy: false,           // 使用 Composition API 模式
  locale: defaultLocale,
  fallbackLocale: 'zh-CN',
  messages: {
    'zh-CN': zhCN,
    'en-US': enUS,
  },
  // 禁止控制台警告：允许在中文 locale 中回退到 key 本身
  silentTranslationWarn: true,
})

export default i18n

/** 切换语言，同时持久化到 localStorage */
export function setLanguage(lang: 'zh-CN' | 'en-US') {
  ;(i18n.global.locale as any).value = lang
  localStorage.setItem('lang', lang)
}
