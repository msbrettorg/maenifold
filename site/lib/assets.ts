import path from 'path'
import fs from 'fs'

export interface Workflow {
  id: string
  name: string
  emoji: string
  description: string
  triggers?: string[]
  enhancedThinkingEnabled?: boolean
  steps?: Array<{
    id: string
    name: string
    description: string
    requiresEnhancedThinking: boolean
  }>
}

export interface Role {
  id: string
  name: string
  emoji: string
  description: string
  triggers?: string[]
  personality?: {
    motto?: string
    principles?: string[]
  }
}

export interface Color {
  id: string
  name: string
  emoji: string
  description: string
  instruction?: string
}

export interface Perspective {
  id: string
  name: string
  emoji: string
  description: string
  instruction?: string
}

/**
 * Load all workflow JSON files
 */
export function getAllWorkflows(): Workflow[] {
  const workflowsDir = path.join(process.cwd(), '../src/assets/workflows')

  if (!fs.existsSync(workflowsDir)) {
    console.warn(`Workflows directory not found: ${workflowsDir}`)
    return []
  }

  const files = fs.readdirSync(workflowsDir).filter((file) => file.endsWith('.json'))

  return files.map((file) => {
    const filePath = path.join(workflowsDir, file)
    const content = fs.readFileSync(filePath, 'utf-8')
    return JSON.parse(content) as Workflow
  }).sort((a, b) => a.name.localeCompare(b.name))
}

/**
 * Load all role JSON files
 */
export function getAllRoles(): Role[] {
  const rolesDir = path.join(process.cwd(), '../src/assets/roles')

  if (!fs.existsSync(rolesDir)) {
    console.warn(`Roles directory not found: ${rolesDir}`)
    return []
  }

  const files = fs.readdirSync(rolesDir).filter((file) => file.endsWith('.json'))

  return files.map((file) => {
    const filePath = path.join(rolesDir, file)
    const content = fs.readFileSync(filePath, 'utf-8')
    return JSON.parse(content) as Role
  }).sort((a, b) => a.name.localeCompare(b.name))
}

/**
 * Load all color JSON files
 */
export function getAllColors(): Color[] {
  const colorsDir = path.join(process.cwd(), '../src/assets/colors')

  if (!fs.existsSync(colorsDir)) {
    console.warn(`Colors directory not found: ${colorsDir}`)
    return []
  }

  const files = fs.readdirSync(colorsDir).filter((file) => file.endsWith('.json'))

  return files.map((file) => {
    const filePath = path.join(colorsDir, file)
    const content = fs.readFileSync(filePath, 'utf-8')
    return JSON.parse(content) as Color
  }).sort((a, b) => a.name.localeCompare(b.name))
}

/**
 * Load all perspective JSON files
 */
export function getAllPerspectives(): Perspective[] {
  const perspectivesDir = path.join(process.cwd(), '../src/assets/perspectives')

  if (!fs.existsSync(perspectivesDir)) {
    console.warn(`Perspectives directory not found: ${perspectivesDir}`)
    return []
  }

  const files = fs.readdirSync(perspectivesDir).filter((file) => file.endsWith('.json'))

  return files.map((file) => {
    const filePath = path.join(perspectivesDir, file)
    const content = fs.readFileSync(filePath, 'utf-8')
    return JSON.parse(content) as Perspective
  }).sort((a, b) => a.name.localeCompare(b.name))
}
