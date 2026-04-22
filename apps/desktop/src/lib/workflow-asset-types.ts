/**
 * TypeScript types for workflow asset definitions.
 * Models the JSON returned by ListAssets (summaries) and ReadMcpResource (full definitions).
 */

// --- ListAssets response shape ---

export interface WorkflowAssetSummary {
  id: string
  name: string
  emoji: string
  description: string
  multiAgent: boolean
  category?: WorkflowCategory
}

// --- ReadMcpResource full definition ---

export interface WorkflowAssetDefinition {
  id: string
  name: string
  emoji: string
  description: string
  triggers: string[]
  sequentialThinkingEnabled: boolean
  multiAgent: boolean
  category?: WorkflowCategory
  steps: WorkflowStepDefinition[]
}

export interface WorkflowStepDefinition {
  id: string
  name: string
  description: string
  requiresSequentialThinking: boolean
  metadata?: WorkflowStepMetadata
}

export interface WorkflowStepMetadata {
  toolHints?: Record<string, ToolHint>
  reasoning_effort?: ReasoningEffort
  stop_conditions?: string[]
  next_actions?: string[]
  escape_hatches?: string[]
}

export interface ToolHint {
  reason: string
  priority: ToolHintPriority
  estimated_thoughts?: number
}

export type ToolHintPriority = 'mandatory' | 'recommended' | 'optional'
export type ReasoningEffort = 'low' | 'medium' | 'high'

export type WorkflowCategory =
  | 'reasoning'
  | 'scientific'
  | 'multi-agent'
  | 'devops'
  | 'memory'
  | 'custom'

export const VALID_CATEGORIES: ReadonlySet<WorkflowCategory> = new Set([
  'reasoning',
  'scientific',
  'multi-agent',
  'devops',
  'memory',
  'custom',
])

const CATEGORY_KEYWORDS: Record<WorkflowCategory, string[]> = {
  reasoning: [
    'reasoning', 'abductive', 'bayesian', 'deductive', 'inductive',
    'dialectic', 'inference', 'logic', 'conjecture', 'falsification',
    'hypothesis', 'analytic', 'socratic',
  ],
  scientific: [
    'scientific', 'experiment', 'method', 'research', 'empirical',
    'observation', 'systematic', 'investigation',
  ],
  'multi-agent': [
    'think-tank', 'game-theory', 'multi-agent', 'debate',
    'adversarial', 'collaborative', 'red-team', 'blue-team',
  ],
  devops: [
    'agentic', 'sdlc', 'dev', 'code', 'deploy', 'ci', 'pipeline',
    'sprint', 'review', 'build', 'test',
  ],
  memory: [
    'memory', 'graph', 'repair', 'hygiene', 'sync', 'concept',
    'knowledge', 'ontology', 'taxonomy',
  ],
  custom: [],
}

export function categorizeWorkflow(wf: WorkflowAssetSummary): WorkflowCategory {
  // Trust the explicit category only when it is a known category — defensive
  // against malformed summaries that might smuggle through an arbitrary
  // string. T-205.711.
  if (wf.category && VALID_CATEGORIES.has(wf.category)) return wf.category
  if (wf.multiAgent) return 'multi-agent'

  const haystack = `${wf.id} ${wf.name} ${wf.description}`.toLowerCase()

  const scores: Partial<Record<WorkflowCategory, number>> = {}
  for (const [cat, keywords] of Object.entries(CATEGORY_KEYWORDS) as [WorkflowCategory, string[]][]) {
    if (cat === 'custom') continue
    const hits = keywords.filter((kw) => haystack.includes(kw)).length
    if (hits > 0) scores[cat] = hits
  }

  if (Object.keys(scores).length === 0) return 'custom'

  return Object.entries(scores).sort(([, a], [, b]) => b - a)[0][0] as WorkflowCategory
}
