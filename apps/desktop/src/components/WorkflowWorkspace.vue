<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { Brain } from 'lucide-vue-next'
import { usePipelineStore } from '../stores/pipeline'
import { useWorkflowLibraryStore } from '../stores/workflow-library'
import EmptyState from './EmptyState.vue'
import WorkflowBrowser from './WorkflowBrowser.vue'
import WorkflowStepTimeline from './WorkflowStepTimeline.vue'
import WorkflowStepInspector from './WorkflowStepInspector.vue'
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from '@/components/ui/resizable'

const pipelineStore = usePipelineStore()
const store = useWorkflowLibraryStore()
const { currentDefinition, currentId, draftDefinition } = storeToRefs(store)
const selectedStepId = ref<string | null>(null)

const displayedDefinition = computed(() => draftDefinition.value ?? currentDefinition.value)
const isWorkflowSelected = computed(() => currentId.value !== null)

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

  store.fetchSummaries()
})
</script>

<template>
  <div role="main" aria-label="Workflow editor" class="flex h-full w-full overflow-hidden">
    <ResizablePanelGroup
      direction="horizontal"
      auto-save-id="workflow-workspace"
      class="h-full w-full"
    >
      <ResizablePanel
        id="browser"
        :default-size="20"
        :min-size="15"
        :max-size="30"
        :order="1"
      >
        <WorkflowBrowser class="h-full overflow-y-auto bg-surface" />
      </ResizablePanel>
      <ResizableHandle
        class="w-px shrink-0 cursor-col-resize bg-border after:w-4 after:bg-transparent after:transition-colors hover:after:bg-accent/70 focus-visible:after:bg-accent/70 data-[state=drag]:after:bg-accent"
      />
      <ResizablePanel
        id="middle"
        :default-size="isWorkflowSelected ? 50 : 80"
        :min-size="30"
        :order="2"
      >
        <div
          v-if="!isWorkflowSelected"
          class="grid h-full place-items-center overflow-hidden"
        >
          <EmptyState
            :icon="Brain"
            title="Select a workflow"
            description="Choose a workflow from the browser to view its steps and details."
          />
        </div>
        <WorkflowStepTimeline
          v-else
          class="h-full overflow-y-auto bg-bg-app"
          :selected-step-id="selectedStepId"
          @save="handleSave"
          @select-step="handleSelectStep"
        />
      </ResizablePanel>
      <template v-if="isWorkflowSelected">
        <ResizableHandle
          class="w-px shrink-0 cursor-col-resize bg-border after:w-4 after:bg-transparent after:transition-colors hover:after:bg-accent/70 focus-visible:after:bg-accent/70 data-[state=drag]:after:bg-accent"
        />
        <ResizablePanel
          id="inspector"
          :default-size="30"
          :min-size="15"
          :order="3"
          collapsible
        >
          <div class="h-full w-full min-w-0 min-h-0">
            <WorkflowStepInspector
              class="h-full overflow-y-auto bg-surface"
              :selected-step-id="selectedStepId"
            />
          </div>
        </ResizablePanel>
      </template>
    </ResizablePanelGroup>
  </div>
</template>
