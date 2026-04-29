<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { Brain, Loader2 } from 'lucide-vue-next'
import { usePipelineStore } from '../stores/pipeline'
import { useWorkflowLibraryStore } from '../stores/workflow-library'
import EmptyState from './EmptyState.vue'
import WorkflowBrowser from './WorkflowBrowser.vue'
import WorkflowLiveProgress from './WorkflowLiveProgress.vue'
import WorkflowStepTimeline from './WorkflowStepTimeline.vue'
import WorkflowStepInspector from './WorkflowStepInspector.vue'
import { useWorkflowStore } from '../stores/workflow'

const pipelineStore = usePipelineStore()
const store = useWorkflowLibraryStore()
const workflowStore = useWorkflowStore()
const {
  currentDefinition,
  currentId,
  draftDefinition,
  filteredWorkflows,
  loading,
  summaries,
} = storeToRefs(store)
const selectedStepId = ref<string | null>(null)

const displayedDefinition = computed(() => draftDefinition.value ?? currentDefinition.value)
const showLiveProgress = computed(() => (
  workflowStore.activeRunWorkflowId === currentId.value &&
  workflowStore.selectedSessionId !== null
))
const isSelectionVisibleInFilter = computed(() => (
  currentId.value !== null &&
  filteredWorkflows.value.some((workflow) => workflow.id === currentId.value)
))
const isInitialLoad = computed(() => loading.value && summaries.value.length === 0)

function handleSelectStep(stepId: string) {
  selectedStepId.value = selectedStepId.value === stepId ? null : stepId
}

function handleSave() {
  void store.saveDraft()
}

watch(currentId, () => {
  selectedStepId.value = null
})

watch(displayedDefinition, (definition) => {
  if (!definition || !selectedStepId.value) return
  if (!definition.steps.some((step: { id: string }) => step.id === selectedStepId.value)) {
    selectedStepId.value = null
  }
})

onMounted(async () => {
  if (!pipelineStore.connected) {
    try {
      await pipelineStore.connect()
    } catch {
      // watcher will retry.
    }
  }

  await store.fetchSummaries()
  await store.ensureVisibleWorkflowSelected()
})
</script>

<template>
  <div role="main" aria-label="Workflow editor" class="flex h-full w-full overflow-hidden">
    <WorkflowBrowser class="h-full shrink-0 overflow-y-auto bg-surface" />
    <section class="flex min-w-0 flex-1 overflow-hidden border-l border-border" aria-label="Workflow detail">
      <div
        v-if="!isSelectionVisibleInFilter && isInitialLoad"
        class="grid h-full min-w-0 flex-1 place-items-center overflow-hidden"
        role="status"
        aria-label="Loading workflows"
      >
        <Loader2 class="h-6 w-6 animate-spin text-text-secondary" aria-hidden="true" />
      </div>
      <div
        v-else-if="!isSelectionVisibleInFilter"
        class="grid h-full min-w-0 flex-1 place-items-center overflow-hidden"
      >
        <EmptyState
          :icon="Brain"
          title="Select a workflow"
          description="Choose a workflow from the browser to view its steps and details."
        />
      </div>
      <WorkflowLiveProgress
        v-else-if="showLiveProgress"
        class="h-full min-w-0 flex-1"
        :workflow-name="displayedDefinition?.name ?? currentId ?? 'Workflow'"
        :workflow="displayedDefinition"
      />
      <template v-else>
        <WorkflowStepTimeline
          class="h-full min-w-0 flex-1 overflow-y-auto bg-bg-app"
          :selected-step-id="selectedStepId"
          @save="handleSave"
          @select-step="handleSelectStep"
        />
        <WorkflowStepInspector
          class="h-full w-80 shrink-0 overflow-y-auto border-l border-border bg-surface"
          :selected-step-id="selectedStepId"
        />
      </template>
    </section>
  </div>
</template>
