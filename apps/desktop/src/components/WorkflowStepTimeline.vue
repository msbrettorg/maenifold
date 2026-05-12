<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { Button } from '@/components/ui/button'
import { useWorkflowLibraryStore } from '../stores/workflow-library'
import WorkflowDraftBar from './WorkflowDraftBar.vue'
import WorkflowExportBar from './WorkflowExportBar.vue'
import WorkflowStepTimelineEntry from './WorkflowStepTimelineEntry.vue'
import WorkflowStepTimelineHeader from './WorkflowStepTimelineHeader.vue'
import { ScrollArea } from '@/components/ui/scroll-area'

const props = defineProps<{
  selectedStepId: string | null
}>()

const emit = defineEmits<{
  'select-step': [stepId: string]
  save: []
  'add-step': []
}>()

const store = useWorkflowLibraryStore()
const {
  currentDefinition,
  definitionLoading,
  draftDefinition,
  exportError, exportState,
  isDirty,
  origin,
  pendingWorkflowId,
  saveError,
  saveState,
} = storeToRefs(store)

const displayedDefinition = computed(() => draftDefinition.value ?? currentDefinition.value)

const expandedStepIds = ref<Set<string>>(new Set())

const allExpanded = computed(() => {
  const def = displayedDefinition.value
  if (!def || def.steps.length === 0) return false
  return def.steps.every((step: { id: string }) => expandedStepIds.value.has(step.id))
})

function isExpanded(stepId: string): boolean {
  return expandedStepIds.value.has(stepId)
}

function toggleExpanded(stepId: string) {
  const next = new Set(expandedStepIds.value)
  if (next.has(stepId)) {
    next.delete(stepId)
  } else {
    next.add(stepId)
  }
  expandedStepIds.value = next
}

function toggleAllExpanded() {
  const def = displayedDefinition.value
  if (!def) return
  if (allExpanded.value) {
    expandedStepIds.value = new Set()
  } else {
    expandedStepIds.value = new Set(def.steps.map((step: { id: string }) => step.id))
  }
}

function expandStep(stepId: string) {
  if (expandedStepIds.value.has(stepId)) return
  const next = new Set(expandedStepIds.value)
  next.add(stepId)
  expandedStepIds.value = next
}

function handleSelectStep(stepId: string) {
  expandStep(stepId)
  emit('select-step', stepId)
}

watch(displayedDefinition, (def) => {
  if (!def) {
    expandedStepIds.value = new Set()
    return
  }
  const stepIds = new Set(def.steps.map((step: { id: string }) => step.id))
  let changed = false
  const next = new Set<string>()
  for (const id of expandedStepIds.value) {
    if (stepIds.has(id)) {
      next.add(id)
    } else {
      changed = true
    }
  }
  if (changed) expandedStepIds.value = next
})

const saveButtonLabel = computed(() => {
  if (saveState.value === 'saving') return 'Saving…'
  if (saveState.value === 'error') return 'Retry'
  if (!isDirty.value) return 'Saved'
  return origin.value === 'packaged' ? 'Save Copy' : 'Save'
})

const exportPromptVisible = ref(false); const lastExportVersion = ref<'draft' | 'saved'>('draft')
let exportFeedbackTimer: ReturnType<typeof setTimeout> | null = null

function handleDiscard() {
  if (pendingWorkflowId.value) {
    void store.confirmDiscardAndContinue()
    return
  }
  store.discardDraft()
}

function handleAddStep() {
  const stepId = store.addStep()
  if (stepId) {
    handleSelectStep(stepId)
  }
}

watch(() => props.selectedStepId, (stepId) => {
  if (stepId) expandStep(stepId)
}, { immediate: true })

function getReferenceWarning(stepId: string) {
  if (!displayedDefinition.value) return null
  const references = displayedDefinition.value.steps
    .filter(
      (step: { id: string; metadata?: unknown }) =>
        step.id !== stepId && JSON.stringify(step.metadata ?? {}).includes(stepId),
    )
    .map((step: { name: string }) => step.name)
  return references.length > 0
    ? `Referenced in ${references.join(', ')} metadata.`
    : null
}

function handleRemoveStep(stepId: string) {
  if (store.removeStep(stepId) && props.selectedStepId === stepId) {
    emit('select-step', stepId)
  }
}

function handleMoveStepUp(stepId: string) { store.moveStepUp(stepId) }
function handleMoveStepDown(stepId: string) { store.moveStepDown(stepId) }

async function exportWorkflow(version: 'draft' | 'saved') {
  exportPromptVisible.value = false
  lastExportVersion.value = version
  await store.exportWorkflow(version)
}

function handleExportClick() {
  if (!displayedDefinition.value || exportState.value === 'exporting') return
  if (isDirty.value) {
    exportPromptVisible.value = true
    return
  }
  void exportWorkflow('draft')
}

watch(displayedDefinition, () => {
  exportPromptVisible.value = false
})

watch(exportState, (state) => {
  if (exportFeedbackTimer) {
    clearTimeout(exportFeedbackTimer)
    exportFeedbackTimer = null
  }
  if (state === 'success') {
    exportFeedbackTimer = setTimeout(() => {
      exportState.value = 'idle'
      exportFeedbackTimer = null
    }, 2000)
  }
})
</script>

<template>
  <div
    v-if="definitionLoading"
    class="flex items-center justify-center h-full min-h-[200px] p-6"
  >
    <span class="text-text-secondary text-xs leading-relaxed">Loading…</span>
  </div>

  <div
    v-else-if="!displayedDefinition"
    class="flex items-center justify-center h-full min-h-[200px] p-6"
  >
    <div class="flex flex-col items-center gap-2 max-w-[340px] text-center">
      <span class="text-text-primary text-sm font-semibold">Choose a workflow</span>
      <span class="text-text-secondary text-xs leading-relaxed">Select a workflow from the browser to review its steps and edit its details.</span>
    </div>
  </div>

  <section v-else class="bg-bg-app min-h-full" aria-label="Workflow step timeline">
    <ScrollArea class="h-full">
    <div class="flex flex-col gap-3 p-4">
    <WorkflowStepTimelineHeader
      :title="displayedDefinition.name"
      :workflow="displayedDefinition"
      :has-steps="displayedDefinition.steps.length > 0"
      :all-expanded="allExpanded"
      :export-state="exportState"
      :is-dirty="isDirty"
      :save-state="saveState"
      :save-button-label="saveButtonLabel"
      @toggle-all="toggleAllExpanded"
      @export-click="handleExportClick"
      @save="emit('save')"
      @duplicate="void store.duplicateCurrentWorkflow()"
      :duplicate-disabled="!store.canDuplicate"
    />

    <WorkflowExportBar
      :export-prompt-visible="exportPromptVisible"
      :export-state="exportState"
      :export-error="exportError"
      :last-export-version="lastExportVersion"
      @export="exportWorkflow"
      @cancel="exportPromptVisible = false"
    />

    <p
      v-if="saveError"
      class="m-0 text-status-abandoned text-xs leading-snug"
      role="alert"
    >{{ saveError }}</p>

    <div class="flex items-center gap-3 flex-wrap font-mono text-xs text-text-secondary tracking-wider">
      <span>{{ displayedDefinition.steps.length }} steps</span>
      <span
        v-if="displayedDefinition.sequentialThinkingEnabled"
        class="pl-3 border-l border-border-subtle"
      >
        Sequential thinking
      </span>
    </div>

    <ol class="list-none m-0 p-0 flex flex-col gap-3">
      <WorkflowStepTimelineEntry
        v-for="(step, index) in displayedDefinition.steps"
        :key="step.id"
        :step="step"
        :index="index"
        :total-steps="displayedDefinition.steps.length"
        :selected-step-id="props.selectedStepId"
        :expanded="isExpanded(step.id)"
        :reference-warning="getReferenceWarning(step.id)"
        @move-down="handleMoveStepDown"
        @move-up="handleMoveStepUp"
        @remove="handleRemoveStep"
        @select="handleSelectStep"
        @toggle-expanded="toggleExpanded"
      />
    </ol>

    <div class="flex gap-3 items-center group">
      <div class="flex flex-col items-center w-10 shrink-0">
        <span class="w-2 h-2 rounded bg-text-secondary shrink-0 transition-colors duration-150 group-hover:bg-node-workflow" />
      </div>
      <Button
        variant="ghost"
        type="button"
        class="flex-1 flex items-center justify-center gap-2 w-full p-3 bg-transparent border border-dashed border-border-subtle rounded-sm text-text-secondary font-mono text-xs tracking-wider hover:border-node-workflow hover:text-node-workflow hover:bg-node-workflow/10"
        @click="handleAddStep"
      >
        ＋ Add new step
      </Button>
    </div>

    <Transition
      enter-active-class="transition duration-[250ms] ease-out"
      enter-from-class="translate-y-full opacity-0"
      leave-active-class="transition duration-150 ease-in"
      leave-to-class="translate-y-full opacity-0"
    >
      <WorkflowDraftBar
        v-if="isDirty || pendingWorkflowId !== null"
        :has-pending-selection="pendingWorkflowId !== null"
        :is-dirty="isDirty"
        :origin="origin"
        :save-message="saveError"
        :save-state="saveState"
        @cancel-pending="store.cancelPendingSelection()"
        @discard="handleDiscard"
        @save="emit('save')"
      />
    </Transition>
    </div>
    </ScrollArea>
  </section>
</template>
