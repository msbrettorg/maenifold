import type {
  ReasoningEffort,
  WorkflowAssetDefinition,
  WorkflowAssetSummary,
  WorkflowCategory,
  WorkflowStepDefinition,
  WorkflowStepMetadata,
} from './workflow-asset-types'
import { VALID_CATEGORIES } from './workflow-asset-types'

const VALID_REASONING_EFFORTS: ReadonlySet<ReasoningEffort> = new Set(['low', 'medium', 'high'])
const VALID_TOOL_HINT_PRIORITIES: ReadonlySet<string> = new Set(['mandatory', 'recommended', 'optional'])

export interface InvalidWorkflowAsset {
  id: string
  reason: string
}

function asRecord(value: unknown): Record<string, unknown> | null {
  return value && typeof value === 'object' ? (value as Record<string, unknown>) : null
}

function requireString(record: Record<string, unknown>, key: string, label: string): string {
  const value = record[key]
  if (typeof value !== 'string' || value.trim().length === 0) {
    throw new Error(`${label} is required.`)
  }
  return value
}

function requireBoolean(record: Record<string, unknown>, key: string, label: string): boolean {
  if (typeof record[key] !== 'boolean') {
    throw new Error(`${label} must be true or false.`)
  }
  return record[key] as boolean
}

function parseCategory(value: unknown): WorkflowCategory | undefined {
  if (value === undefined) return undefined
  if (typeof value !== 'string' || !VALID_CATEGORIES.has(value as WorkflowCategory)) {
    throw new Error('Workflow category is invalid.')
  }
  return value as WorkflowCategory
}

function parseToolHints(value: unknown) {
  const hints = asRecord(value)
  if (!hints) throw new Error('Step metadata.toolHints must be an object.')
  for (const [toolName, hint] of Object.entries(hints)) {
    const hintRecord = asRecord(hint)
    if (!hintRecord) throw new Error(`Tool hint '${toolName}' must be an object.`)
    if (typeof hintRecord.reason !== 'string' || hintRecord.reason.trim().length === 0) {
      throw new Error(`Tool hint '${toolName}' reason is required.`)
    }
    if (
      typeof hintRecord.priority !== 'string' ||
      !VALID_TOOL_HINT_PRIORITIES.has(hintRecord.priority)
    ) {
      throw new Error(`Tool hint '${toolName}' priority is invalid.`)
    }
    if (
      hintRecord.estimated_thoughts !== undefined &&
      (typeof hintRecord.estimated_thoughts !== 'number' || !Number.isFinite(hintRecord.estimated_thoughts))
    ) {
      throw new Error(`Tool hint '${toolName}' estimated_thoughts must be a number.`)
    }
  }
}

function parseMetadata(value: unknown): WorkflowStepMetadata | undefined {
  if (value === undefined) return undefined
  const metadata = asRecord(value)
  if (!metadata) throw new Error('Step metadata must be an object.')
  if (metadata.toolHints !== undefined) parseToolHints(metadata.toolHints)
  if (
    metadata.reasoning_effort !== undefined &&
    (typeof metadata.reasoning_effort !== 'string' ||
      !VALID_REASONING_EFFORTS.has(metadata.reasoning_effort as ReasoningEffort))
  ) {
    throw new Error('Step metadata.reasoning_effort must be low, medium, or high.')
  }
  return metadata as WorkflowStepMetadata
}

function parseStep(stepValue: unknown, index: number): WorkflowStepDefinition {
  const step = asRecord(stepValue)
  if (!step) throw new Error(`Step ${index + 1} must be an object.`)
  return {
    id: requireString(step, 'id', `Step ${index + 1} id`),
    name: requireString(step, 'name', `Step ${index + 1} name`),
    description: requireString(step, 'description', `Step ${index + 1} description`),
    requiresSequentialThinking: requireBoolean(
      step,
      'requiresSequentialThinking',
      `Step ${index + 1} requiresSequentialThinking`,
    ),
    metadata: parseMetadata(step.metadata),
  }
}

export function filterWorkflowSummaries(input: unknown): {
  valid: WorkflowAssetSummary[]
  invalid: InvalidWorkflowAsset[]
} {
  if (!Array.isArray(input)) {
    return {
      valid: [],
      invalid: [{ id: 'workflow-summaries', reason: 'ListAssets did not return an array.' }],
    }
  }

  const seenIds = new Set<string>()
  return input.reduce<{ valid: WorkflowAssetSummary[]; invalid: InvalidWorkflowAsset[] }>(
    (result, entry, index) => {
      const record = asRecord(entry)
      const id = record && typeof record.id === 'string' && record.id.length > 0
        ? record.id
        : `workflow-${index + 1}`
      try {
        if (!record) throw new Error('Workflow summary must be an object.')
        const validatedId = requireString(record, 'id', 'Workflow ID')
        // T-205.701: drop duplicate workflow summaries (same id appearing twice
        // from the data source). Keep the first occurrence so library list
        // entries remain unique.
        if (seenIds.has(validatedId)) {
          return result
        }
        seenIds.add(validatedId)
        const category = parseCategory(record.category)
        result.valid.push({
          id: validatedId,
          name: typeof record.name === 'string' && record.name.trim() ? record.name : id,
          emoji: typeof record.emoji === 'string' && record.emoji.trim() ? record.emoji : '⚙️',
          description: typeof record.description === 'string' ? record.description : '',
          multiAgent: typeof record.multiAgent === 'boolean' ? record.multiAgent : false,
          ...(category ? { category } : {}),
        })
      } catch (error) {
        result.invalid.push({
          id,
          reason: error instanceof Error ? error.message : String(error),
        })
      }
      return result
    },
    { valid: [], invalid: [] },
  )
}

export function validateWorkflowDefinition(input: unknown): WorkflowAssetDefinition {
  const record = asRecord(input)
  if (!record) throw new Error('Workflow definition must be an object.')
  if (!Array.isArray(record.steps) || record.steps.length === 0) {
    throw new Error('Workflow must include at least one step.')
  }

  const category = parseCategory(record.category)
  if (!Array.isArray(record.triggers)) {
    record.triggers = []
  } else {
    record.triggers = record.triggers.filter((entry: unknown) => typeof entry === 'string')
  }

  requireString(record, 'id', 'Workflow ID')
  if (typeof record.name !== 'string' || !record.name.trim()) record.name = record.id as string
  if (typeof record.emoji !== 'string' || !record.emoji.trim()) record.emoji = '⚙️'
  if (typeof record.description !== 'string') record.description = ''
  if (typeof record.sequentialThinkingEnabled !== 'boolean') record.sequentialThinkingEnabled = false
  if (typeof record.multiAgent !== 'boolean') record.multiAgent = false
  record.steps.map(parseStep)
  if (category === undefined) delete record.category
  return input as WorkflowAssetDefinition
}
