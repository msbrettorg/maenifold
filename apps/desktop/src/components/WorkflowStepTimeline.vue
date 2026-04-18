<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { Button } from '@/components/ui/button'
import { useWorkflowLibraryStore } from '../stores/workflow-library'
import WorkflowDraftBar from './WorkflowDraftBar.vue'
import WorkflowStepCard from './WorkflowStepCard.vue'

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
  exportError,
  exportState,
  isDirty,
  origin,
  pendingWorkflowId,
  saveError,
  saveState,
} =
  storeToRefs(store)

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

function handleSelectStep(stepId: string) {
  // Selecting a step also expands it so the inspector context matches the timeline.
  if (!expandedStepIds.value.has(stepId)) {
    const next = new Set(expandedStepIds.value)
    next.add(stepId)
    expandedStepIds.value = next
  }
  emit('select-step', stepId)
}

watch(displayedDefinition, (def) => {
  if (!def) {
    expandedStepIds.value = new Set()
    return
  }
  // Drop ids that no longer exist after edits.
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
const saveButtonClass = computed(() => ({
  'timeline-save-button--dirty': isDirty.value,
  'timeline-save-button--error': saveState.value === 'error',
}))
const exportPromptVisible = ref(false)
const lastExportVersion = ref<'draft' | 'saved'>('draft')
let exportFeedbackTimer: ReturnType<typeof setTimeout> | null = null

function formatStepNumber(index: number): string {
  return String(index + 1).padStart(2, '0')
}

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
    emit('select-step', stepId)
  }
}

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
  <div v-if="definitionLoading" class="timeline-empty">
    <span class="empty-text">Loading…</span>
  </div>

  <div v-else-if="!displayedDefinition" class="timeline-empty">
    <div class="empty-state">
      <span class="empty-title">Choose a workflow</span>
      <span class="empty-text">Select a workflow from the browser to review its steps and edit its details.</span>
    </div>
  </div>

  <section v-else class="timeline" aria-label="Workflow step timeline">
    <header class="timeline-header">
      <h1 class="timeline-title">
        <span class="title-emoji">{{ displayedDefinition.emoji }}</span>
        {{ displayedDefinition.name }}
      </h1>
      <div class="header-actions">
        <Button
          variant="outline"
          class="timeline-fold-button"
          :disabled="!displayedDefinition || displayedDefinition.steps.length === 0"
          @click="toggleAllExpanded"
        >
          {{ allExpanded ? 'Collapse all' : 'Expand all' }}
        </Button>
        <Button variant="outline" :disabled="exportState === 'exporting'" @click="handleExportClick">
          {{ exportState === 'exporting' ? 'Exporting…' : 'Export' }}
        </Button>
        <Button
          variant="outline"
          :class="['timeline-save-button', saveButtonClass]"
          :disabled="saveState === 'saving' || (!isDirty && saveState !== 'error')"
          @click="emit('save')"
        >
          {{ saveButtonLabel }}
        </Button>

      </div>
    </header>

    <div v-if="exportPromptVisible || exportState !== 'idle'" class="export-feedback" role="status" aria-live="polite">
      <template v-if="exportPromptVisible">
        <span>Export draft or saved version?</span>
        <button class="feedback-btn" type="button" @click="exportWorkflow('draft')">Draft</button>
        <button class="feedback-btn" type="button" @click="exportWorkflow('saved')">Saved</button>
        <button class="feedback-btn" type="button" @click="exportPromptVisible = false">Cancel</button>
      </template>
      <template v-else-if="exportState === 'error'">
        <span>{{ exportError ?? 'Export failed.' }}</span>
        <button class="feedback-btn" type="button" @click="exportWorkflow(lastExportVersion)">Retry</button>
      </template>
      <template v-else-if="exportState === 'success'">
        <span>Exported ✓</span>
      </template>
    </div>

    <p v-if="saveError" class="save-feedback" role="alert">{{ saveError }}</p>


    <div class="timeline-meta">
      <span>{{ displayedDefinition.steps.length }} steps</span>
      <span v-if="displayedDefinition.sequentialThinkingEnabled" class="meta-sep">
        Sequential thinking
      </span>
    </div>

    <ol class="step-list">
      <li
        v-for="(step, index) in displayedDefinition.steps"
        :key="step.id"
        class="step-entry"
      >
        <div class="step-rail">
          <span
            :class="['step-number', { 'step-number--selected': props.selectedStepId === step.id }]"
          >
            {{ formatStepNumber(index) }}
          </span>
          <span v-if="index < displayedDefinition.steps.length - 1" class="step-connector" />
        </div>

        <WorkflowStepCard
          :can-remove="displayedDefinition.steps.length > 1"
          :can-move-down="index < displayedDefinition.steps.length - 1"
          :can-move-up="index > 0"
          :expanded="isExpanded(step.id)"
          :reference-warning="getReferenceWarning(step.id)"
          :selected="props.selectedStepId === step.id"
          :step="step"
          @move-down="handleMoveStepDown"
          @move-up="handleMoveStepUp"
          @remove="handleRemoveStep"
          @select="handleSelectStep"
          @toggle-expanded="toggleExpanded"
        />
      </li>
    </ol>

    <div class="step-entry add-entry">
      <div class="step-rail">
        <span class="step-dot" />
      </div>
      <button class="add-step-btn" type="button" @click="handleAddStep">
        ＋ Add new step
      </button>
    </div>

    <Transition name="draft-bar">
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
  </section>
</template>

<style scoped>
.timeline-empty { display: flex; align-items: center; justify-content: center; height: 100%; min-height: 200px; padding: var(--space-6); }
.empty-state { display: flex; flex-direction: column; align-items: center; gap: var(--space-2); max-width: 340px; text-align: center; }
.empty-title { color: var(--foreground); font-family: var(--font-display); font-size: var(--text-body); }
.empty-text { color: var(--muted-foreground); font-family: var(--font-body); font-size: var(--text-caption); line-height: 1.6; }
.timeline { display: flex; flex-direction: column; gap: var(--space-3); padding: var(--space-4); background: var(--background); min-height: 100%; overflow-y: auto; }
.timeline-header { display: flex; align-items: flex-start; justify-content: space-between; gap: var(--space-4); padding-bottom: var(--space-3); border-bottom: 1px solid var(--border); }
.timeline-title { margin: 0; font-family: var(--font-display); font-size: var(--text-lg); font-weight: var(--weight-title); color: var(--foreground); display: flex; align-items: center; gap: var(--space-2); min-width: 0; line-height: 1.2; }
.title-emoji { flex-shrink: 0; font-size: var(--text-title); }
.header-actions { display: flex; align-items: center; gap: var(--space-2); flex-shrink: 0; }
.export-feedback { display: flex; align-items: center; flex-wrap: wrap; gap: var(--space-2); color: var(--muted-foreground); font: var(--weight-display) var(--text-tiny)/1.4 var(--font-body); }
.feedback-btn { min-height: 24px; padding: 0 var(--space-2); border: 1px solid var(--border); border-radius: var(--radius-sm); background: transparent; color: inherit; font: inherit; cursor: pointer; }
.feedback-btn:hover { background: var(--card); }
.timeline-save-button { color: var(--muted-foreground); opacity: 0.4; cursor: default; }
.timeline-fold-button { color: var(--muted-foreground); }
.timeline-fold-button:not(:disabled):hover { color: var(--color-node-workflow); border-color: color-mix(in srgb, var(--color-node-workflow) 40%, var(--border)); }
.timeline-save-button--dirty { color: var(--color-status-active); border-color: var(--color-status-active); background: color-mix(in srgb, var(--color-status-active) 10%, transparent); opacity: 1; cursor: pointer; }
.timeline-save-button--dirty:hover { background: color-mix(in srgb, var(--color-status-active) 18%, transparent); }
.timeline-save-button--error { color: var(--color-status-abandoned); border-color: var(--color-status-abandoned); background: color-mix(in srgb, var(--color-status-abandoned) 10%, transparent); opacity: 1; cursor: pointer; }
.save-feedback { margin: 0; color: var(--color-status-abandoned); font: var(--weight-display) var(--text-tiny)/1.4 var(--font-body); }
.timeline-meta { display: flex; align-items: center; gap: var(--space-3); flex-wrap: wrap; font-family: var(--font-code); font-size: var(--text-tiny); color: var(--muted-foreground); letter-spacing: 0.05em; }
.meta-sep::before { content: ''; display: inline-block; width: 1px; height: 0.75em; background: var(--border); margin-right: var(--space-3); vertical-align: middle; }
.step-list { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: var(--space-3); }
.step-entry { display: flex; gap: var(--space-3); align-items: stretch; }
.add-entry { align-items: center; }
.step-rail { display: flex; flex-direction: column; align-items: center; width: 40px; flex-shrink: 0; }
.step-number { display: flex; align-items: center; justify-content: center; width: 40px; height: 40px; border-radius: var(--radius); border: 1px solid var(--border); background: var(--card); font-family: var(--font-code); font-size: var(--text-caption); font-weight: var(--weight-title); color: var(--muted-foreground); flex-shrink: 0; transition: border-color var(--transition-fast), color var(--transition-fast), background var(--transition-fast); }
.step-number--selected { border-color: var(--color-node-workflow); color: var(--color-node-workflow); background: color-mix(in srgb, var(--color-node-workflow) 10%, var(--card)); }
.step-connector { flex: 1; width: 1px; min-height: var(--space-3); border-left: 2px dashed var(--border); }
.step-dot { width: 8px; height: 8px; border-radius: var(--radius); background: var(--muted-foreground); flex-shrink: 0; transition: background var(--transition-fast); }
.add-step-btn { flex: 1; display: flex; align-items: center; justify-content: center; gap: var(--space-2); width: 100%; padding: var(--space-3); background: transparent; border: 1px dashed var(--border); border-radius: var(--radius-sm); color: var(--muted-foreground); font-family: var(--font-code); font-size: var(--text-tiny); letter-spacing: 0.05em; cursor: pointer; transition: border-color var(--transition-fast), color var(--transition-fast), background var(--transition-fast); }
.add-step-btn:hover { border-color: var(--color-node-workflow); color: var(--color-node-workflow); background: color-mix(in srgb, var(--color-node-workflow) 6%, transparent); }
.add-entry:hover .step-dot { background: var(--color-node-workflow); }
.draft-bar-enter-active { transition: transform var(--transition-normal), opacity var(--transition-normal); }
.draft-bar-enter-from { transform: translateY(100%); opacity: 0; }
.draft-bar-leave-active { transition: transform var(--transition-fast), opacity var(--transition-fast); }
.draft-bar-leave-to { transform: translateY(100%); opacity: 0; }
</style>
