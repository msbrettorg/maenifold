<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
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
const saveButtonLabel = computed(() => {
  if (saveState.value === 'saving') return 'Saving…'
  if (saveState.value === 'error') return 'Retry'
  if (!isDirty.value) return 'Saved'
  return origin.value === 'packaged' ? 'Save Copy' : 'Save'
})
const saveButtonClass = computed(() => ({
  'btn--save-dirty': isDirty.value,
  'btn--save-error': saveState.value === 'error',
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
    <span class="empty-text">Select a workflow</span>
  </div>

  <section v-else class="timeline" aria-label="Workflow step timeline">
    <header class="timeline-header">
      <h1 class="timeline-title">
        <span class="title-emoji">{{ displayedDefinition.emoji }}</span>
        {{ displayedDefinition.name }}
      </h1>
      <div class="header-actions">
        <button class="btn btn--ghost" :disabled="exportState === 'exporting'" type="button" @click="handleExportClick">
          {{ exportState === 'exporting' ? 'Exporting…' : 'Export' }}
        </button>
        <button
          :class="['btn btn--save', saveButtonClass]"
          :disabled="saveState === 'saving' || (!isDirty && saveState !== 'error')"
          type="button"
          @click="emit('save')"
        >
          {{ saveButtonLabel }}
        </button>

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
      <span>{{ displayedDefinition.steps.length }} STEPS</span>
      <span v-if="displayedDefinition.sequentialThinkingEnabled" class="meta-sep">
        SEQUENTIAL THINKING
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
          :reference-warning="getReferenceWarning(step.id)"
          :selected="props.selectedStepId === step.id"
          :step="step"
          @move-down="handleMoveStepDown"
          @move-up="handleMoveStepUp"
          @remove="handleRemoveStep"
          @select="emit('select-step', $event)"
        />
      </li>
    </ol>

    <div class="step-entry add-entry">
      <div class="step-rail">
        <span class="step-dot" />
      </div>
      <button class="add-step-btn" type="button" @click="handleAddStep">
        ＋ ADD NEW STEP
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
.timeline-empty { display: flex; align-items: center; justify-content: center; height: 100%; min-height: 200px; }
.empty-text { color: var(--text-muted); font-family: var(--font-headline); font-size: var(--text-base); }
.timeline { display: flex; flex-direction: column; gap: var(--space-3); padding: var(--space-4); background: var(--bg-main); min-height: 100%; overflow-y: auto; }
.timeline-header { display: flex; align-items: flex-start; justify-content: space-between; gap: var(--space-4); padding-bottom: var(--space-3); border-bottom: 1px solid var(--border-default); }
.timeline-title { margin: 0; font-family: var(--font-headline); font-size: var(--text-lg); font-weight: 600; color: var(--text-primary); display: flex; align-items: center; gap: var(--space-2); min-width: 0; line-height: 1.2; }
.title-emoji { flex-shrink: 0; font-size: var(--text-xl); }
.header-actions { display: flex; align-items: center; gap: var(--space-2); flex-shrink: 0; }
.export-feedback { display: flex; align-items: center; flex-wrap: wrap; gap: var(--space-2); color: var(--text-secondary); font: 500 var(--text-xs)/1.4 var(--font-sans); }
.feedback-btn { min-height: 24px; padding: 0 var(--space-2); border: 1px solid var(--border-default); border-radius: var(--radius-sm); background: transparent; color: inherit; font: inherit; cursor: pointer; }
.feedback-btn:hover { background: var(--surface-container); }
.btn { display: inline-flex; align-items: center; gap: var(--space-1); min-height: 32px; padding: 0 var(--space-3); border-radius: var(--radius-sm); border: 1px solid transparent; font-family: var(--font-sans); font-size: var(--text-sm); font-weight: 500; cursor: pointer; transition: background var(--transition-fast), border-color var(--transition-fast), box-shadow var(--transition-fast), opacity var(--transition-fast); white-space: nowrap; }
.btn--ghost { background: transparent; color: var(--text-primary); border-color: var(--border-default); }
.btn--ghost:hover { background: var(--surface-container); }
.btn--save { background: transparent; color: var(--text-muted); border-color: var(--border-default); opacity: 0.4; cursor: default; }
.btn--save-dirty { color: var(--color-status-active); border-color: var(--color-status-active); background: color-mix(in srgb, var(--color-status-active) 10%, transparent); opacity: 1; cursor: pointer; }
.btn--save-dirty:hover { background: color-mix(in srgb, var(--color-status-active) 18%, transparent); }
.btn--save-error { color: var(--color-status-abandoned); border-color: var(--color-status-abandoned); background: color-mix(in srgb, var(--color-status-abandoned) 10%, transparent); opacity: 1; cursor: pointer; }
.save-feedback { margin: 0; color: var(--color-status-abandoned); font: 500 var(--text-xs)/1.4 var(--font-sans); }
.btn:disabled { pointer-events: none; }
.timeline-meta { display: flex; align-items: center; gap: var(--space-3); flex-wrap: wrap; font-family: var(--font-mono); font-size: var(--text-xs); color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.05em; }
.meta-sep::before { content: ''; display: inline-block; width: 1px; height: 0.75em; background: var(--border-default); margin-right: var(--space-3); vertical-align: middle; }
.step-list { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: var(--space-3); }
.step-entry { display: flex; gap: var(--space-3); align-items: stretch; }
.add-entry { align-items: center; }
.step-rail { display: flex; flex-direction: column; align-items: center; width: 40px; flex-shrink: 0; }
.step-number { display: flex; align-items: center; justify-content: center; width: 40px; height: 40px; border-radius: 50%; border: 1px solid var(--border-default); background: var(--bg-panel); font-family: var(--font-mono); font-size: var(--text-sm); font-weight: 600; color: var(--text-muted); flex-shrink: 0; transition: border-color var(--transition-fast), color var(--transition-fast), background var(--transition-fast); }
.step-number--selected { border-color: var(--color-node-workflow); color: var(--color-node-workflow); background: color-mix(in srgb, var(--color-node-workflow) 10%, var(--bg-panel)); }
.step-connector { flex: 1; width: 1px; min-height: var(--space-3); border-left: 2px dashed var(--border-default); }
.step-dot { width: 8px; height: 8px; border-radius: 50%; background: var(--text-muted); flex-shrink: 0; transition: background var(--transition-fast); }
.add-step-btn { flex: 1; display: flex; align-items: center; justify-content: center; gap: var(--space-2); width: 100%; padding: var(--space-3); background: transparent; border: 1px dashed var(--border-default); border-radius: var(--radius-sm); color: var(--text-muted); font-family: var(--font-mono); font-size: var(--text-xs); letter-spacing: 0.05em; text-transform: uppercase; cursor: pointer; transition: border-color var(--transition-fast), color var(--transition-fast), background var(--transition-fast); }
.add-step-btn:hover { border-color: var(--color-node-workflow); color: var(--color-node-workflow); background: color-mix(in srgb, var(--color-node-workflow) 6%, transparent); }
.add-entry:hover .step-dot { background: var(--color-node-workflow); }
.draft-bar-enter-active { transition: transform var(--transition-normal), opacity var(--transition-normal); }
.draft-bar-enter-from { transform: translateY(100%); opacity: 0; }
.draft-bar-leave-active { transition: transform var(--transition-fast), opacity var(--transition-fast); }
.draft-bar-leave-to { transform: translateY(100%); opacity: 0; }
</style>
