export type Theme = 'light' | 'dark'

type StorageLike = {
  theme?: unknown
  getItem?: (key: string) => string | null
}

type WindowLike = {
  localStorage?: StorageLike
  matchMedia?: (query: string) => { matches: boolean }
}

type DocumentLike = {
  documentElement?: {
    classList?: {
      add: (token: string) => void
      remove: (token: string) => void
    }
  }
}

const readStoredTheme = (storage?: StorageLike): Theme | null => {
  const storedTheme =
    typeof storage?.getItem === 'function' ? storage.getItem('theme') : storage?.theme

  return storedTheme === 'light' || storedTheme === 'dark' ? storedTheme : null
}

export const getInitialThemeFrom = (win?: WindowLike): Theme => {
  const storedTheme = readStoredTheme(win?.localStorage)

  if (storedTheme) {
    return storedTheme
  }

  return win?.matchMedia?.('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
}

export const applyThemeTo = (doc: DocumentLike | undefined, theme: Theme): void => {
  if (!doc?.documentElement?.classList) {
    return
  }

  if (theme === 'dark') {
    doc.documentElement.classList.add('dark')
    return
  }

  doc.documentElement.classList.remove('dark')
}

export const getInitialTheme = (): Theme =>
  typeof window === 'undefined' ? 'light' : getInitialThemeFrom(window)

export const applyTheme = (theme: Theme): void => {
  if (typeof document === 'undefined') {
    return
  }

  applyThemeTo(document, theme)
}
