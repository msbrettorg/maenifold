import type { Ref } from 'vue'
import type {
  ReasoningEffort,
  WorkflowAssetDefinition,
  WorkflowCategory,
} from '../lib/workflow-asset-types'
import { asRecord, cloneDefinition, deepClone, type WorkflowRaw } from './workflow-library-helpers'

interface DraftActionContext {
  draftDefinition: Ref<WorkflowAssetDefinition | null>
  rawJson: Ref<WorkflowRaw | null>
  beginEditing: () => WorkflowAssetDefinition | null
  onMutate?: () => void
}

export function createWorkflowLibraryDraftActions(context: DraftActionContext) {
  function ensureDraft() {
    if (!context.draftDefinition.value) context.beginEditing()
    return context.draftDefinition.value
  }

  function updateStep(stepId: string, mutate: (definition: WorkflowAssetDefinition) => void) {
    const draft = ensureDraft()
    if (!draft || !draft.steps.some((step: { id: string }) => step.id === stepId)) return
    context.onMutate?.()
    const nextDraft = cloneDefinition(draft)
    mutate(nextDraft)
    context.draftDefinition.value = nextDraft
  }

  function updateRawSteps(nextSteps: unknown[]) {
    const nextRaw = deepClone(context.rawJson.value ?? {}) as WorkflowRaw
    nextRaw.steps = nextSteps
    context.rawJson.value = nextRaw
  }

  function updateWorkflow(mutate: (definition: WorkflowAssetDefinition) => void) {
    const draft = ensureDraft()
    if (!draft) return
    context.onMutate?.()
    const nextDraft = cloneDefinition(draft)
    mutate(nextDraft)
    context.draftDefinition.value = nextDraft
  }

  function updateStepName(stepId: string, name: string) {
    updateStep(stepId, (definition) => {
      const step = definition.steps.find((entry: { id: string }) => entry.id === stepId)
      if (step) step.name = name
    })
  }

  function updateStepDescription(stepId: string, description: string) {
    updateStep(stepId, (definition) => {
      const step = definition.steps.find((entry: { id: string }) => entry.id === stepId)
      if (step) step.description = description
    })
  }

  function updateWorkflowCategory(category: WorkflowCategory | null) {
    updateWorkflow((definition) => {
      if (category) {
        definition.category = category
        return
      }

      delete definition.category
    })
  }

  function updateWorkflowDescription(description: string) {
    updateWorkflow((definition) => {
      definition.description = description
    })
  }

  function updateWorkflowEmoji(emoji: string) {
    updateWorkflow((definition) => {
      definition.emoji = emoji
    })
  }

  function updateWorkflowMultiAgent(value: boolean) {
    updateWorkflow((definition) => {
      definition.multiAgent = value
    })
  }

  function updateWorkflowName(name: string) {
    updateWorkflow((definition) => {
      definition.name = name
    })
  }

  function updateWorkflowSequentialThinkingEnabled(value: boolean) {
    updateWorkflow((definition) => {
      definition.sequentialThinkingEnabled = value
    })
  }

  function updateWorkflowTriggers(value: string[]) {
    updateWorkflow((definition) => {
      definition.triggers = [...value]
    })
  }

  function setStepRequiresSequentialThinking(stepId: string, value: boolean) {
    updateStep(stepId, (definition) => {
      const step = definition.steps.find((entry: { id: string }) => entry.id === stepId)
      if (step) step.requiresSequentialThinking = value
    })
  }

  function setStepReasoningEffort(stepId: string, value: ReasoningEffort) {
    updateStep(stepId, (definition) => {
      const step = definition.steps.find((entry: { id: string }) => entry.id === stepId)
      if (!step) return
      step.metadata = {
        ...(step.metadata ?? {}),
        reasoning_effort: value,
      }
    })
  }

  function addStep() {
    const draft = ensureDraft()
    if (!draft) return null
    context.onMutate?.()
    const stepId = crypto.randomUUID()
    const newStep = {
      id: stepId,
      name: 'New Step',
      description: 'Describe what this step does.',
      requiresSequentialThinking: false,
      metadata: {
        reasoning_effort: 'medium',
      },
    } as const
    const nextDraft = cloneDefinition(draft)
    nextDraft.steps.push({ ...newStep })
    context.draftDefinition.value = nextDraft
    updateRawSteps([
      ...(Array.isArray(context.rawJson.value?.steps) ? (context.rawJson.value.steps as unknown[]) : []),
      deepClone(newStep),
    ])
    return stepId
  }

  function removeStep(stepId: string) {
    const draft = ensureDraft()
    if (!draft || draft.steps.length <= 1) return false
    const removeIndex = draft.steps.findIndex((step: { id: string }) => step.id === stepId)
    if (removeIndex === -1) return false
    context.onMutate?.()
    const nextDraft = cloneDefinition(draft)
    nextDraft.steps.splice(removeIndex, 1)
    context.draftDefinition.value = nextDraft
    updateRawSteps(
      Array.isArray(context.rawJson.value?.steps)
        ? (context.rawJson.value.steps as unknown[]).filter((entry) => asRecord(entry)?.id !== stepId)
        : [],
    )
    return true
  }

  function moveStep(stepId: string, direction: -1 | 1) {
    const draft = ensureDraft()
    if (!draft) return false
    const currentIndex = draft.steps.findIndex((step: { id: string }) => step.id === stepId)
    const targetIndex = currentIndex + direction
    if (currentIndex < 0 || targetIndex < 0 || targetIndex >= draft.steps.length) return false
    context.onMutate?.()
    const nextDraft = cloneDefinition(draft)
    ;[nextDraft.steps[currentIndex], nextDraft.steps[targetIndex]] = [
      nextDraft.steps[targetIndex],
      nextDraft.steps[currentIndex],
    ]
    context.draftDefinition.value = nextDraft

    const rawSteps = Array.isArray(context.rawJson.value?.steps)
      ? [...(context.rawJson.value.steps as unknown[])]
      : []
    ;[rawSteps[currentIndex], rawSteps[targetIndex]] = [rawSteps[targetIndex], rawSteps[currentIndex]]
    updateRawSteps(rawSteps)
    return true
  }

  return {
    addStep,
    moveStepDown: (stepId: string) => moveStep(stepId, 1),
    moveStepUp: (stepId: string) => moveStep(stepId, -1),
    removeStep,
    setStepReasoningEffort,
    setStepRequiresSequentialThinking,
    updateStepDescription,
    updateStepName,
    updateWorkflowCategory,
    updateWorkflowDescription,
    updateWorkflowEmoji,
    updateWorkflowMultiAgent,
    updateWorkflowName,
    updateWorkflowSequentialThinkingEnabled,
    updateWorkflowTriggers,
  }
}
