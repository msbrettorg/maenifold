import type { Ref } from 'vue'
import type { WorkflowAssetDefinition, WorkflowAssetSummary } from '../lib/workflow-asset-types'
import { getNotificationSink } from '@/lib/notification-sink';
import { usePipelineMcp } from '../composables/usePipelineMcp'
import { validateWorkflowDefinition } from '../lib/workflow-asset-runtime'
import {
  cloneDefinition,
  mergeDraftIntoRaw,
  type WorkflowOrigin,
  type WorkflowRaw,
} from './workflow-library-helpers'
import {
  IMPORT_WORKFLOW_PENDING_ID,
  NEW_WORKFLOW_PENDING_ID,
} from './workflow-library-session-actions'

export type WorkflowSaveState = 'idle' | 'saving' | 'error' | 'success'
export type WorkflowExportState = 'idle' | 'exporting' | 'error' | 'success'

interface SaveActionContext {
  applyPendingImport: () => boolean
  createNewWorkflow: (force?: boolean) => boolean
  currentDefinition: Ref<WorkflowAssetDefinition | null>
  currentId: Ref<string | null>
  draftDefinition: Ref<WorkflowAssetDefinition | null>
  exportError: Ref<string | null>
  exportState: Ref<WorkflowExportState>
  origin: Ref<WorkflowOrigin | null>
  pendingWorkflowId: Ref<string | null>
  rawJson: Ref<WorkflowRaw | null>
  saveError: Ref<string | null>
  saveState: Ref<WorkflowSaveState>
  summaries: Ref<WorkflowAssetSummary[]>
  fetchSummaries: () => Promise<void>
  loadWorkflow: (id: string) => Promise<void>
  selectWorkflow: (id: string) => Promise<void>
}

function validateDraft(draft: WorkflowAssetDefinition): string | null {
  if (!draft.name.trim()) return 'Workflow name is required.'
  if (!/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(draft.id)) {
    return 'Workflow ID must use kebab-case.'
  }
  if (draft.steps.length === 0) return 'Workflow must include at least one step.'
  return null
}

function downloadWorkflowJson(content: string) {
  const blob = new Blob([content], { type: 'application/json' })
  const url = URL.createObjectURL(blob)
  const popup = window.open(url, '_blank', 'noopener,noreferrer')
  if (!popup) {
    URL.revokeObjectURL(url)
    throw new Error('Unable to open export in a new window. Please allow popups and try again.')
  }

  window.setTimeout(() => {
    URL.revokeObjectURL(url)
  }, 10_000)
}

export function createWorkflowLibrarySaveActions(context: SaveActionContext) {
  const workflowMcp = usePipelineMcp()

  async function exportWorkflow(version: 'draft' | 'saved' = 'draft') {
    if (!context.rawJson.value || !context.currentDefinition.value) return false

    context.exportState.value = 'exporting'
    context.exportError.value = null

    try {
      const sourceDefinition =
        version === 'saved'
          ? context.currentDefinition.value
          : (context.draftDefinition.value ?? context.currentDefinition.value)
      const merged = mergeDraftIntoRaw(context.rawJson.value, sourceDefinition)
      downloadWorkflowJson(`${JSON.stringify(merged, null, 2)}\n`)
      context.exportState.value = 'success'
      return true
    } catch (error) {
      context.exportError.value = error instanceof Error ? error.message : String(error)
      context.exportState.value = 'error'
      return false
    }
  }

  async function saveDraft() {
    if (!context.rawJson.value || !context.draftDefinition.value) return false

    const validationError = validateDraft(context.draftDefinition.value)
    if (validationError) {
      context.saveError.value = validationError
      context.saveState.value = 'error'
      return false
    }

    context.saveState.value = 'saving'
    context.saveError.value = null

    try {
      const nextDraft = cloneDefinition(context.draftDefinition.value)
      if (context.origin.value === 'packaged') {
        nextDraft.id = nextDraft.id.endsWith('-custom') ? nextDraft.id : `${nextDraft.id}-custom`
      }

      const merged = mergeDraftIntoRaw(context.rawJson.value, nextDraft)
      if (!workflowMcp.connected.value) {
        await workflowMcp.connect()
      }
      await workflowMcp.gatewayExecute('maenifold.workflow', 'SaveWorkflowAsset', {
        id: nextDraft.id,
        content: `${JSON.stringify(merged, null, 2)}\n`,
      })

      const persisted: WorkflowRaw = {
        ...merged,
        uri: `asset://workflows/${nextDraft.id}`,
        source: 'user',
        workflowOrigin: 'user',
        origin: 'user',
      }
      delete persisted.path

      context.rawJson.value = persisted
      context.currentDefinition.value = cloneDefinition(nextDraft)
      context.draftDefinition.value = cloneDefinition(nextDraft)
      context.currentId.value = nextDraft.id
      context.origin.value = 'user'
      context.saveState.value = 'success'
      await context.fetchSummaries()

      const nextId = context.pendingWorkflowId.value
      context.pendingWorkflowId.value = null
      if (nextId === NEW_WORKFLOW_PENDING_ID) {
        context.createNewWorkflow(true)
      } else if (nextId === IMPORT_WORKFLOW_PENDING_ID) {
        context.applyPendingImport()
      } else if (nextId && nextId !== context.currentId.value) {
        await context.loadWorkflow(nextId)
      }

      return true
    } catch (error) {
      context.saveError.value = error instanceof Error ? error.message : String(error)
      context.saveState.value = 'error'
      return false
    }
  }

  function nextDuplicateId(id: string): string {
    let suffix = 1
    let candidate = `${id}-copy`
    while (context.summaries.value.some((w) => w.id === candidate)) {
      suffix += 1
      candidate = `${id}-copy-${suffix}`
    }
    return candidate
  }

  async function duplicateCurrentWorkflow(): Promise<string | null> {
    if (!context.currentId.value || !context.rawJson.value) return null
    const copyId = nextDuplicateId(context.currentId.value)
    const raw = { ...context.rawJson.value }
    raw.id = copyId
    raw.name = typeof raw.name === 'string' && raw.name.trim() ? `${raw.name} Copy` : copyId
    const content = `${JSON.stringify(raw, null, 2)}\n`
    try {
      if (!workflowMcp.connected.value) await workflowMcp.connect()
      await workflowMcp.gatewayExecute('maenifold.workflow', 'SaveWorkflowAsset', { id: copyId, content })
    } catch (err) {
      getNotificationSink().error(err instanceof Error ? err.message : String(err))
      return null
    }
    await context.fetchSummaries()
    await context.selectWorkflow(copyId)
    return copyId
  }

  async function duplicateWorkflowById(id: string): Promise<string | null> {
    const source = context.summaries.value.find((workflow) => workflow.id === id)
    if (!source) return null
    const copyId = nextDuplicateId(source.id)

    try {
      if (!workflowMcp.connected.value) await workflowMcp.connect()
      const sourceRaw = validateWorkflowDefinition(
        await workflowMcp.readWorkflowDefinition(source.id),
      ) as unknown as WorkflowRaw
      const raw = { ...sourceRaw }
      raw.id = copyId
      raw.name = typeof raw.name === 'string' && raw.name.trim() ? `${raw.name} Copy` : copyId
      const content = `${JSON.stringify(raw, null, 2)}\n`
      await workflowMcp.gatewayExecute('maenifold.workflow', 'SaveWorkflowAsset', { id: copyId, content })
    } catch (err) {
      getNotificationSink().error(err instanceof Error ? err.message : String(err))
      return null
    }

    await context.fetchSummaries()
    getNotificationSink().success(`Duplicated as ${copyId}`)
    return copyId
  }

  return {
    exportWorkflow,
    saveDraft,
    duplicateCurrentWorkflow,
    duplicateWorkflowById,
  }
}
