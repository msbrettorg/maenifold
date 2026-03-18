import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'

declare global {
  interface Window {
    __pinia?: string
  }
}

const app = createApp(App)
const pinia = createPinia()

if (typeof window !== 'undefined' && window.__pinia) {
  pinia.state.value = JSON.parse(window.__pinia)
}

app.use(pinia)
app.mount('#app')
