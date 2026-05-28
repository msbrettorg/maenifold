import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import path from 'node:path'
import ts from 'typescript'

const projectRoot = path.resolve(new URL('.', import.meta.url).pathname, '..')
const themePath = path.join(projectRoot, 'src/lib/theme.ts')

const source = await readFile(themePath, 'utf8')
const { outputText } = ts.transpileModule(source, {
  compilerOptions: {
    module: ts.ModuleKind.ES2022,
    target: ts.ScriptTarget.ES2022,
  },
  fileName: themePath,
})

const themeModule = await import(
  `data:text/javascript;base64,${Buffer.from(outputText).toString('base64')}`,
)

const { applyThemeTo, getInitialThemeFrom } = themeModule

const createWindow = ({ storedTheme, prefersDark }) => ({
  localStorage: {
    theme: storedTheme,
    getItem(key) {
      return key === 'theme' ? storedTheme ?? null : null
    },
  },
  matchMedia(query) {
    assert.equal(query, '(prefers-color-scheme: dark)')
    return { matches: prefersDark }
  },
})

const createDocument = () => {
  const classes = new Set()

  return {
    classes,
    documentElement: {
      classList: {
        add(name) {
          classes.add(name)
        },
        remove(name) {
          classes.delete(name)
        },
        contains(name) {
          return classes.has(name)
        },
      },
    },
  }
}

try {
  assert.equal(
    getInitialThemeFrom(createWindow({ storedTheme: 'dark', prefersDark: false })),
    'dark',
  )
  assert.equal(
    getInitialThemeFrom(createWindow({ storedTheme: 'light', prefersDark: true })),
    'light',
  )
  assert.equal(getInitialThemeFrom(createWindow({ prefersDark: true })), 'dark')
  assert.equal(getInitialThemeFrom(createWindow({ prefersDark: false })), 'light')

  const doc = createDocument()
  applyThemeTo(doc, 'dark')
  assert.equal(doc.documentElement.classList.contains('dark'), true)
  applyThemeTo(doc, 'light')
  assert.equal(doc.documentElement.classList.contains('dark'), false)

  process.exitCode = 0
} catch (error) {
  console.error(error)
  process.exitCode = 1
}
