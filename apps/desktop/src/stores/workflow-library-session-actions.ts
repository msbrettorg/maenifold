import type { ComputedRef, Ref } from 'vue'
import type {
  WorkflowAssetDefinition,
  WorkflowAssetSummary,
  WorkflowCategory,
  WorkflowStepDefinition,
} from '../lib/workflow-asset-types'
import { asRecord, cloneDefinition, deepClone, type WorkflowOrigin, type WorkflowRaw } from './workflow-library-helpers'

export const NEW_WORKFLOW_PENDING_ID = '__new-workflow__'
export const IMPORT_WORKFLOW_PENDING_ID = '__import-workflow__'

export type WorkflowImportResolution = 'overwrite' | 'copy'
export type WorkflowImportResult =
  | { status: 'success' }
  | { status: 'pending' }
  | { status: 'conflict', id: string, name: string }
  | { status: 'error', message: string }

export interface PendingImportedWorkflow {
  definition: WorkflowAssetDefinition
  raw: WorkflowRaw
}

interface SessionActionContext {
  currentDefinition: Ref<WorkflowAssetDefinition | null>
  currentId: Ref<string | null>
  draftDefinition: Ref<WorkflowAssetDefinition | null>
  error: Ref<string | null>
  exportError: Ref<string | null>
  exportState: Ref<'idle' | 'exporting' | 'error' | 'success'>
  isDirty: ComputedRef<boolean>
  origin: Ref<WorkflowOrigin | null>
  pendingImportedWorkflow: Ref<PendingImportedWorkflow | null>
  pendingWorkflowId: Ref<string | null>
  rawJson: Ref<WorkflowRaw | null>
  saveError: Ref<string | null>
  saveState: Ref<'idle' | 'saving' | 'error' | 'success'>
  summaries: Ref<WorkflowAssetSummary[]>
}

const VALID_CATEGORIES: ReadonlySet<WorkflowCategory> = new Set([
  'reasoning',
  'scientific',
  'multi-agent',
  'devops',
  'memory',
  'custom',
])
const VALID_REASONING_EFFORTS = new Set(['low', 'medium', 'high'])

function requireBoolean(record: WorkflowRaw, key: string, label: string) {
  if (typeof record[key] !== 'boolean') throw new Error(`${label} must be true or false.`)
  return record[key] as boolean
}

function requireString(record: WorkflowRaw, key: string, label: string) {
  if (typeof record[key] !== 'string' || !record[key].trim()) throw new Error(`${label} is required.`)
  return record[key] as string
}

function parseTriggers(record: WorkflowRaw) {
  if (!Array.isArray(record.triggers) || !record.triggers.every((entry) => typeof entry === 'string')) {
    throw new Error('Workflow triggers must be an array of strings.')
  }
  return [...record.triggers]
}

function parseMetadata(step: WorkflowRaw) {
  if (step.metadata === undefined) return undefined
  const metadata = asRecord(step.metadata)
  if (!metadata) throw new Error('Step metadata must be an object.')
  if (
    metadata.reasoning_effort !== undefined &&
    (typeof metadata.reasoning_effort !== 'string' || !VALID_REASONING_EFFORTS.has(metadata.reasoning_effort))
  ) {
    throw new Error('Step metadata.reasoning_effort must be low, medium, or high.')
  }
  return deepClone(metadata) as WorkflowStepDefinition['metadata']
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
    metadata: parseMetadata(step),
  }
}

function parseImportedWorkflow(json: string): PendingImportedWorkflow {
  let parsed: unknown
  try {
    parsed = JSON.parse(json)
  } catch {
    throw new Error('Workflow file is not valid JSON.')
  }

  const raw = asRecord(parsed)
  if (!raw) throw new Error('Workflow file must be a JSON object.')
  if (!Array.isArray(raw.steps) || raw.steps.length === 0) {
    throw new Error('Workflow must include at least one step.')
  }

  const category = raw.category
  if (
    category !== undefined &&
    (typeof category !== 'string' || !VALID_CATEGORIES.has(category as WorkflowCategory))
  ) {
    throw new Error('Workflow category is invalid.')
  }

  return {
    raw: deepClone(raw),
    definition: {
      id: requireString(raw, 'id', 'Workflow ID'),
      name: requireString(raw, 'name', 'Workflow name'),
      emoji: requireString(raw, 'emoji', 'Workflow emoji'),
      description: requireString(raw, 'description', 'Workflow description'),
      triggers: parseTriggers(raw),
      sequentialThinkingEnabled: requireBoolean(
        raw,
        'sequentialThinkingEnabled',
        'Workflow sequentialThinkingEnabled',
      ),
      multiAgent: requireBoolean(raw, 'multiAgent', 'Workflow multiAgent'),
      ...(category ? { category: category as WorkflowCategory } : {}),
      steps: raw.steps.map(parseStep),
    },
  }
}

function nextCopyId(id: string, summaries: WorkflowAssetSummary[]) {
  let suffix = 1
  let candidate = `${id}-copy`
  while (summaries.some((workflow) => workflow.id === candidate)) {
    suffix += 1
    candidate = `${id}-copy-${suffix}`
  }
  return candidate
}

export function createWorkflowLibrarySessionActions(context: SessionActionContext) {
  function resetSessionState() {
    context.error.value = null
    context.exportError.value = null
    context.exportState.value = 'idle'
    context.saveError.value = null
    context.saveState.value = 'idle'
  }

  function applyDraftWorkflow(definition: WorkflowAssetDefinition, raw: WorkflowRaw) {
    context.currentId.value = definition.id
    context.currentDefinition.value = null
    context.draftDefinition.value = cloneDefinition(definition)
    context.rawJson.value = deepClone(raw)
    context.origin.value = 'draft'
    context.pendingWorkflowId.value = null
    context.pendingImportedWorkflow.value = null
    resetSessionState()
  }

  function createNewWorkflow(force = false) {
    if (context.isDirty.value && context.currentId.value && !force) {
      context.pendingWorkflowId.value = NEW_WORKFLOW_PENDING_ID
      return false
    }

    const definition: WorkflowAssetDefinition = {
      id: crypto.randomUUID(),
      name: 'Untitled Workflow',
      emoji: '⚙️',
      description: '',
      triggers: [],
      sequentialThinkingEnabled: false,
      multiAgent: false,
      steps: [{
        id: crypto.randomUUID(),
        name: 'New Step',
        description: 'Describe what this step does.',
        requiresSequentialThinking: false,
        metadata: { reasoning_effort: 'medium' },
      }],
    }
    applyDraftWorkflow(definition, deepClone(definition) as unknown as WorkflowRaw)
    return true
  }

  function applyPendingImport() {
    if (!context.pendingImportedWorkflow.value) return false
    const pendingImport = context.pendingImportedWorkflow.value
    applyDraftWorkflow(pendingImport.definition, pendingImport.raw)
    return true
  }

  function importWorkflow(
    json: string,
    resolution?: WorkflowImportResolution,
    force = false,
  ): WorkflowImportResult {
    try {
      const pendingImport = parseImportedWorkflow(json)
      const existing = context.summaries.value.find((workflow) => workflow.id === pendingImport.definition.id)
      if (existing && !resolution) {
        return { status: 'conflict', id: existing.id, name: existing.name }
      }

      if (existing && resolution === 'copy') {
        const copyId = nextCopyId(pendingImport.definition.id, context.summaries.value)
        pendingImport.definition.id = copyId
        pendingImport.definition.name = `${pendingImport.definition.name} Copy`
        pendingImport.raw.id = copyId
        pendingImport.raw.name = pendingImport.definition.name
      }

      if (context.isDirty.value && context.currentId.value && !force) {
        context.pendingImportedWorkflow.value = pendingImport
        context.pendingWorkflowId.value = IMPORT_WORKFLOW_PENDING_ID
        return { status: 'pending' }
      }

      applyDraftWorkflow(pendingImport.definition, pendingImport.raw)
      return { status: 'success' }
    } catch (error) {
      return {
        status: 'error',
        message: error instanceof Error ? error.message : String(error),
      }
    }
  }

  function clearPendingImport() {
    context.pendingImportedWorkflow.value = null
  }

  return {
    applyPendingImport,
    clearPendingImport,
    createNewWorkflow,
    importWorkflow,
  }
}
