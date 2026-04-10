import type { WorkflowAssetDefinition } from '../lib/workflow-asset-types'

export type WorkflowRaw = Record<string, unknown>
export type WorkflowOrigin = 'packaged' | 'user' | 'draft'

export function deepClone<T>(value: T): T {
  try {
    return deepClone(value)
  } catch {
    return JSON.parse(JSON.stringify(value))
  }
}

export const cloneDefinition = (definition: WorkflowAssetDefinition): WorkflowAssetDefinition =>
  deepClone(definition)

export const asRecord = (value: unknown): WorkflowRaw | null =>
  value && typeof value === 'object' ? (value as WorkflowRaw) : null

function normalizeForCompare(value: unknown): unknown {
  if (Array.isArray(value)) {
    return value.map(normalizeForCompare)
  }

  if (value && typeof value === 'object') {
    return Object.keys(value as Record<string, unknown>)
      .sort()
      .reduce<Record<string, unknown>>((result, key) => {
        result[key] = normalizeForCompare((value as Record<string, unknown>)[key])
        return result
      }, {})
  }

  return value
}

export function definitionsEqual(
  left: WorkflowAssetDefinition | null,
  right: WorkflowAssetDefinition | null,
): boolean {
  return JSON.stringify(normalizeForCompare(left)) === JSON.stringify(normalizeForCompare(right))
}

export function detectWorkflowOrigin(raw: WorkflowRaw, fallbackId: string): WorkflowOrigin {
  void fallbackId
  const metadataOrigin = [raw.workflowOrigin, raw.origin, raw.source]
    .find((value): value is string => typeof value === 'string' && value.trim().length > 0)
    ?.trim()
    .toLowerCase()

  if (metadataOrigin === 'draft') {
    return 'draft'
  }

  if (metadataOrigin === 'user' || metadataOrigin === 'custom') {
    return 'user'
  }

  if (metadataOrigin === 'packaged') {
    return 'packaged'
  }

  return 'packaged'
}

export function mergeDraftIntoRaw(
  raw: WorkflowRaw,
  draft: WorkflowAssetDefinition,
): WorkflowRaw {
  const merged = deepClone(raw)
  const rawSteps = Array.isArray(merged.steps) ? merged.steps : []

  merged.id = draft.id
  merged.name = draft.name
  merged.emoji = draft.emoji
  merged.description = draft.description
  merged.triggers = deepClone(draft.triggers)
  merged.sequentialThinkingEnabled = draft.sequentialThinkingEnabled
  merged.multiAgent = draft.multiAgent
  if (draft.category) {
    merged.category = draft.category
  } else {
    delete merged.category
  }

  merged.steps = draft.steps.map((step, index) => {
    const source =
      rawSteps.find((entry) => asRecord(entry)?.id === step.id) ??
      rawSteps[index] ??
      {}
    const sourceStep = asRecord(source) ?? {}
    const nextStep: WorkflowRaw = {
      ...deepClone(sourceStep),
      id: step.id,
      name: step.name,
      description: step.description,
      requiresSequentialThinking: step.requiresSequentialThinking,
    }
    const nextMetadata = {
      ...(asRecord(sourceStep.metadata) ?? {}),
    }

    if (step.metadata?.reasoning_effort) {
      nextMetadata.reasoning_effort = step.metadata.reasoning_effort
    } else {
      delete nextMetadata.reasoning_effort
    }

    if (Object.keys(nextMetadata).length > 0) {
      nextStep.metadata = nextMetadata
    } else {
      delete nextStep.metadata
    }

    return nextStep
  })

  return merged
}
