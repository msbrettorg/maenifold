import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import '@fontsource-variable/geist'
import './index.css'
import App from './App.tsx'
import { applyTheme, getInitialTheme } from './lib/theme'

const rootElement = document.getElementById('root')

if (!rootElement) {
  throw new Error('Root element not found')
}

applyTheme(getInitialTheme())

createRoot(rootElement).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
