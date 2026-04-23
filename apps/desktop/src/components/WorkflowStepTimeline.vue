<!-- audit-ok: max-loc -->
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
  expandStep(stepId)
  emit('select-step', stepId)
}

function expandStep(stepId: string) {
  if (expandedStepIds.value.has(stepId)) return
  const next = new Set(expandedStepIds.value)
  next.add(stepId)
  expandedStepIds.value = next
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
    <span class="text-[var(--color-text-secondary)] font-[family-name:var(--font-body)] text-xs leading-relaxed">Loading…</span>
  </div>

  <div
    v-else-if="!displayedDefinition"
    class="flex items-center justify-center h-full min-h-[200px] p-6"
  >
    <div class="flex flex-col items-center gap-2 max-w-[340px] text-center">
      <span class="text-[var(--color-text-primary)] font-[family-name:var(--font-display)] text-sm">Choose a workflow</span>
      <span class="text-[var(--color-text-secondary)] font-[family-name:var(--font-body)] text-xs leading-relaxed">Select a workflow from the browser to review its steps and edit its details.</span>
    </div>
  </div>

  <section
    v-else
    class="flex flex-col gap-3 p-4 bg-[var(--color-bg-app)] min-h-full overflow-y-auto"
    aria-label="Workflow step timeline"
  >
    <header class="flex items-start justify-between gap-4 pb-3 bg-[color-mix(in_srgb,var(--color-bg-elevated)_28%,var(--color-bg-app))] rounded-[var(--radius-sm)]">
      <h1 class="m-0 font-[family-name:var(--font-display)] text-[length:var(--text-lg)] font-[number:var(--weight-title)] text-[var(--color-text-primary)] flex items-center gap-2 min-w-0 leading-tight">
        {{ displayedDefinition.name }}
      </h1>
      <div class="flex items-center gap-2 shrink-0">
        <Button
          variant="outline"
          class="text-[var(--color-text-secondary)] [&:not(:disabled):hover]:text-[var(--color-node-workflow)] [&:not(:disabled):hover]:border-[color-mix(in_srgb,var(--color-node-workflow)_40%,var(--color-border-subtle))]"
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
          :class="[
            isDirty
              ? 'text-[var(--color-status-active)] border-[var(--color-status-active)] bg-[color-mix(in_srgb,var(--color-status-active)_10%,transparent)] opacity-100 cursor-pointer hover:bg-[color-mix(in_srgb,var(--color-status-active)_18%,transparent)]'
              : saveState === 'error'
                ? 'text-[var(--color-status-abandoned)] border-[var(--color-status-abandoned)] bg-[color-mix(in_srgb,var(--color-status-abandoned)_10%,transparent)] opacity-100 cursor-pointer'
                : 'text-[var(--color-text-secondary)] opacity-40 cursor-default',
          ]"
          :disabled="saveState === 'saving' || (!isDirty && saveState !== 'error')"
          @click="emit('save')"
        >
          {{ saveButtonLabel }}
        </Button>

      </div>
    </header>

    <div
      v-if="exportPromptVisible || exportState !== 'idle'"
      class="flex items-center flex-wrap gap-2 text-[var(--color-text-secondary)] font-[family-name:var(--font-body)] text-xs font-[number:var(--weight-display)] leading-snug"
      role="status"
      aria-live="polite"
    >
      <template v-if="exportPromptVisible">
        <span>Export draft or saved version?</span>
        <Button
          variant="outline"
          size="xs"
          class="min-h-[24px] px-2 border border-[var(--color-border-subtle)] rounded-[var(--radius-sm)] bg-transparent text-inherit cursor-pointer hover:bg-[var(--color-bg-elevated)]"
          type="button"
          @click="exportWorkflow('draft')"
        >Draft</Button>
        <Button
          variant="outline"
          size="xs"
          class="min-h-[24px] px-2 border border-[var(--color-border-subtle)] rounded-[var(--radius-sm)] bg-transparent text-inherit cursor-pointer hover:bg-[var(--color-bg-elevated)]"
          type="button"
          @click="exportWorkflow('saved')"
        >Saved</Button>
        <Button
          variant="ghost"
          size="xs"
          class="min-h-[24px] px-2 border border-[var(--color-border-subtle)] rounded-[var(--radius-sm)] bg-transparent text-inherit cursor-pointer hover:bg-[var(--color-bg-elevated)]"
          type="button"
          @click="exportPromptVisible = false"
        >Cancel</Button>
      </template>
      <template v-else-if="exportState === 'error'">
        <span>{{ exportError ?? 'Export failed.' }}</span>
        <Button
          variant="outline"
          size="xs"
          class="min-h-[24px] px-2 border border-[var(--color-border-subtle)] rounded-[var(--radius-sm)] bg-transparent text-inherit cursor-pointer hover:bg-[var(--color-bg-elevated)]"
          type="button"
          @click="exportWorkflow(lastExportVersion)"
        >Retry</Button>
      </template>
      <template v-else-if="exportState === 'success'">
        <span>Exported ✓</span>
      </template>
    </div>

    <p
      v-if="saveError"
      class="m-0 text-[var(--color-status-abandoned)] font-[family-name:var(--font-body)] text-xs font-[number:var(--weight-display)] leading-snug"
      role="alert"
    >{{ saveError }}</p>


    <div class="flex items-center gap-3 flex-wrap font-[family-name:var(--font-code)] text-xs text-[var(--color-text-secondary)] tracking-[0.05em]">
      <span>{{ displayedDefinition.steps.length }} steps</span>
      <span
        v-if="displayedDefinition.sequentialThinkingEnabled"
        class="before:content-[''] before:inline-block before:w-px before:h-[0.75em] before:bg-[var(--color-border-subtle)] before:mr-3 before:align-middle"
      >
        Sequential thinking
      </span>
    </div>

    <ol class="list-none m-0 p-0 flex flex-col gap-3">
      <li
        v-for="(step, index) in displayedDefinition.steps"
        :key="step.id"
        class="flex gap-3 items-stretch"
      >
        <div class="flex flex-col items-center w-10 shrink-0">
          <span
            :class="[
              'flex items-center justify-center w-10 h-10 rounded-[var(--radius)] border bg-[var(--color-bg-elevated)] font-[family-name:var(--font-code)] text-xs font-[number:var(--weight-title)] shrink-0 transition-[border-color,color,background] duration-150',
              props.selectedStepId === step.id
                ? 'border-[var(--color-node-workflow)] text-[var(--color-node-workflow)] bg-[color-mix(in_srgb,var(--color-node-workflow)_10%,var(--color-bg-elevated))]'
                : 'border-[var(--color-border-subtle)] text-[var(--color-text-secondary)]',
            ]"
          >
            {{ formatStepNumber(index) }}
          </span>
          <span
            v-if="index < displayedDefinition.steps.length - 1"
            class="flex-1 w-px min-h-3 border-l-2 border-dashed border-[var(--color-border-subtle)]"
          />
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

    <div class="flex gap-3 items-center group">
      <div class="flex flex-col items-center w-10 shrink-0">
        <span class="w-2 h-2 rounded-[var(--radius)] bg-[var(--color-text-secondary)] shrink-0 transition-colors duration-150 group-hover:bg-[var(--color-node-workflow)]" />
      </div>
      <Button
        variant="ghost"
        class="flex-1 flex items-center justify-center gap-2 w-full p-3 bg-transparent border border-dashed border-[var(--color-border-subtle)] rounded-[var(--radius-sm)] text-[var(--color-text-secondary)] font-[family-name:var(--font-code)] text-xs tracking-[0.05em] cursor-pointer transition-[border-color,color,background] duration-150 hover:border-[var(--color-node-workflow)] hover:text-[var(--color-node-workflow)] hover:bg-[color-mix(in_srgb,var(--color-node-workflow)_6%,transparent)]"
        type="button"
        @click="handleAddStep"
      >
        ＋ Add new step
      </Button>
    </div>

    <Transition
      enter-active-class="transition-[transform,opacity] duration-[250ms] ease-[cubic-bezier(0.4,0,0.2,1)]"
      enter-from-class="translate-y-full opacity-0"
      leave-active-class="transition-[transform,opacity] duration-150 ease-[cubic-bezier(0.4,0,0.2,1)]"
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
  </section>
</template>
