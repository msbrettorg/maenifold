import type { Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { homeDir, join } from '@tauri-apps/api/path'
import { save } from '@tauri-apps/plugin-dialog'
import { writeTextFile } from '@tauri-apps/plugin-fs'
import type { WorkflowAssetDefinition } from '../lib/workflow-asset-types'
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
  fetchSummaries: () => Promise<void>
  loadWorkflow: (id: string) => Promise<void>
}

function validateDraft(draft: WorkflowAssetDefinition): string | null {
  if (!draft.name.trim()) return 'Workflow name is required.'
  if (!/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(draft.id)) {
    return 'Workflow ID must use kebab-case.'
  }
  if (draft.steps.length === 0) return 'Workflow must include at least one step.'
  return null
}

async function resolveWorkflowAssetsRoot() {
  return join(await homeDir(), 'maenifold', 'assets', 'workflows')
}

async function resolveSavePath(
  root: string,
  raw: WorkflowRaw,
  id: string,
  origin: WorkflowOrigin | null,
) {
  const currentPath = typeof raw.path === 'string' ? raw.path : null
  if (origin === 'user' && currentPath?.startsWith(root) && typeof raw.id === 'string' && raw.id === id) {
    return currentPath
  }

  return join(root, `${id}.json`)
}

export function createWorkflowLibrarySaveActions(context: SaveActionContext) {
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
      const path = await save({
        defaultPath: `${sourceDefinition.id}.json`,
        filters: [{ name: 'Workflow JSON', extensions: ['json'] }],
        title: 'Export workflow',
      })

      if (!path) {
        context.exportState.value = 'idle'
        return false
      }

      await writeTextFile(path, `${JSON.stringify(merged, null, 2)}\n`)
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
      const root = await resolveWorkflowAssetsRoot()
      const path = await resolveSavePath(root, context.rawJson.value, nextDraft.id, context.origin.value)
      const savedPath = await invoke<string>('save_workflow_asset', {
        json: `${JSON.stringify(merged, null, 2)}\n`,
        path,
      })

      const persisted = {
        ...merged,
        path: savedPath,
      } satisfies WorkflowRaw

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

  return {
    exportWorkflow,
    saveDraft,
  }
}
