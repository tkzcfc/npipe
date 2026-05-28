import { createApp } from 'vue'
import { createPinia } from 'pinia'
import ElementPlus from 'element-plus'
import * as ElementPlusIconsVue from '@element-plus/icons-vue'
// ① 基础样式
import 'element-plus/dist/index.css'
// ② 官方暗色变量
import 'element-plus/theme-chalk/dark/css-vars.css'
import router from './router'
import App from './App.vue'
import i18n from './locales'
import './styles/index.scss'

const app = createApp(App)
const pinia = createPinia()

// Register all Element Plus icons globally
for (const [key, component] of Object.entries(ElementPlusIconsVue)) {
  app.component(key, component)
}

app.use(pinia)
app.use(router)
app.use(i18n)
app.use(ElementPlus, { zIndex: 3000 })

app.mount('#app')

