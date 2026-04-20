<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { Brain } from 'lucide-vue-next'
import { usePipelineStore } from '../stores/pipeline'
import { useWorkflowLibraryStore } from '../stores/workflow-library'
import { getSidebarSizingTokens } from '../lib/sidebarSizing'
import EmptyState from './EmptyState.vue'
import PanelResizeHandle from './PanelResizeHandle.vue'
import WorkflowBrowser from './WorkflowBrowser.vue'
import WorkflowStepTimeline from './WorkflowStepTimeline.vue'
import WorkflowStepInspector from './WorkflowStepInspector.vue'

const STORAGE_KEY = 'workflow-authoring-panel-widths-v2'
const HANDLE_WIDTH = 12
const TIMELINE_MIN = 320
const sidebarSizing = getSidebarSizingTokens()
const BROWSER_MIN = sidebarSizing.minWidth
const BROWSER_DEFAULT = 320
const BROWSER_MAX = sidebarSizing.maxWidth
const INSPECTOR_MIN = 320
const INSPECTOR_DEFAULT = 360

const pipelineStore = usePipelineStore()
const store = useWorkflowLibraryStore()
const { currentDefinition, currentId, draftDefinition } = storeToRefs(store)
const pageRef = ref<HTMLElement | null>(null)
const selectedStepId = ref<string | null>(null)
const browserWidth = ref(BROWSER_DEFAULT)
const inspectorWidth = ref(INSPECTOR_DEFAULT)
const draggingPanel = ref<'browser' | 'inspector' | null>(null)

const displayedDefinition = computed(() => draftDefinition.value ?? currentDefinition.value)
const isWorkflowSelected = computed(() => currentId.value !== null)
const isInspectorVisible = computed(() => selectedStepId.value !== null)

let resizeObserver: ResizeObserver | null = null

function handleSelectStep(stepId: string) {
  selectedStepId.value = selectedStepId.value === stepId ? null : stepId
}

function handleSave() {
  void store.saveDraft()
}

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max)
}

function getAvailableSideWidth() {
  const handleCount = isInspectorVisible.value ? 2 : 1
  return (pageRef.value?.clientWidth ?? 0) - HANDLE_WIDTH * handleCount - TIMELINE_MIN
}

function constrainWidths(nextBrowser: number, nextInspector: number) {
  let browser = Math.min(Math.max(nextBrowser, BROWSER_MIN), BROWSER_MAX)
  let inspector = Math.max(nextInspector, INSPECTOR_MIN)
  const available = getAvailableSideWidth()

  if (available <= 0) return { browser, inspector }
  if (!isInspectorVisible.value) {
    return { browser: Math.min(browser, available), inspector }
  }
  if (browser + inspector <= available) return { browser, inspector }

  let overflow = browser + inspector - available
  const shrinkInspector = Math.min(inspector - INSPECTOR_MIN, overflow)
  inspector -= Math.max(shrinkInspector, 0)
  overflow -= Math.max(shrinkInspector, 0)

  if (overflow > 0) {
    const shrinkBrowser = Math.min(browser - BROWSER_MIN, overflow)
    browser -= Math.max(shrinkBrowser, 0)
  }

  return { browser, inspector }
}

function applyWidths(nextBrowser: number, nextInspector: number) {
  const constrained = constrainWidths(nextBrowser, nextInspector)
  browserWidth.value = constrained.browser
  inspectorWidth.value = constrained.inspector
}

function persistWidths() {
  try {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({ browser: browserWidth.value, inspector: inspectorWidth.value }),
    )
  } catch {
    // localStorage is best-effort only.
  }
}

function restoreWidths() {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (!raw) return

    const parsed = JSON.parse(raw) as Partial<Record<'browser' | 'inspector', unknown>>
    applyWidths(
      typeof parsed.browser === 'number' && Number.isFinite(parsed.browser)
        ? parsed.browser
        : BROWSER_DEFAULT,
      typeof parsed.inspector === 'number' && Number.isFinite(parsed.inspector)
        ? parsed.inspector
        : INSPECTOR_DEFAULT,
    )
  } catch {
    applyWidths(BROWSER_DEFAULT, INSPECTOR_DEFAULT)
  }
}

function handleBrowserResize(delta: number) {
  const maxBrowser = Math.max(
    BROWSER_MIN,
    Math.min(BROWSER_MAX, getAvailableSideWidth() - (isInspectorVisible.value ? inspectorWidth.value : 0)),
  )
  browserWidth.value = clamp(browserWidth.value + delta, BROWSER_MIN, maxBrowser)
}

function handleInspectorResize(delta: number) {
  const maxInspector = Math.max(INSPECTOR_MIN, getAvailableSideWidth() - browserWidth.value)
  inspectorWidth.value = clamp(inspectorWidth.value - delta, INSPECTOR_MIN, maxInspector)
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

watch(isInspectorVisible, (visible) => {
  if (!visible && draggingPanel.value === 'inspector') {
    draggingPanel.value = null
  }
  applyWidths(browserWidth.value, inspectorWidth.value)
})

onMounted(async () => {
  restoreWidths()
  applyWidths(browserWidth.value, inspectorWidth.value)

  if (pageRef.value) {
    resizeObserver = new ResizeObserver(() => {
      applyWidths(browserWidth.value, inspectorWidth.value)
    })
    resizeObserver.observe(pageRef.value)
  }

  if (!pipelineStore.connected) {
    try {
      await pipelineStore.connect()
    } catch {
      // watcher will retry.
    }
  }

  store.fetchSummaries()
})

onBeforeUnmount(() => {
  resizeObserver?.disconnect()
})
</script>

<template>
  <div
    ref="pageRef"
    role="main"
    aria-label="Workflow editor"
    class="workflow-authoring-page"
    :class="{ 'workflow-authoring-page--dragging': draggingPanel !== null }"
  >
    <WorkflowBrowser
      class="panel panel-browser"
      :style="isWorkflowSelected ? { width: `${browserWidth}px` } : undefined"
    />
    <div v-if="!isWorkflowSelected" class="flex flex-1 items-center justify-center overflow-hidden">
      <EmptyState
        :icon="Brain"
        title="Select a workflow"
        description="Choose a workflow from the browser to view its steps and details."
      />
    </div>
    <template v-if="isWorkflowSelected">
      <PanelResizeHandle
        side="left"
        :dragging="draggingPanel === 'browser'"
        @drag-start="draggingPanel = 'browser'"
        @resize="handleBrowserResize"
        @drag-end="draggingPanel = null; persistWidths()"
      />
      <WorkflowStepTimeline
        class="panel panel-timeline"
        :selected-step-id="selectedStepId"
        @save="handleSave"
        @select-step="handleSelectStep"
      />
      <template v-if="isInspectorVisible">
        <PanelResizeHandle
          side="right"
          :dragging="draggingPanel === 'inspector'"
          @drag-start="draggingPanel = 'inspector'"
          @resize="handleInspectorResize"
          @drag-end="draggingPanel = null; persistWidths()"
        />
        <WorkflowStepInspector
          class="panel panel-inspector"
          :selected-step-id="selectedStepId"
          :style="{ width: `${inspectorWidth}px` }"
        />
      </template>
    </template>
  </div>
</template>

<style scoped>
.workflow-authoring-page {
  display: flex;
  height: 100%;
  overflow: hidden;
}

.panel {
  min-height: 0;
  overflow-y: auto;
}

.panel-browser,
.panel-inspector {
  flex: 0 0 auto;
  background: var(--surface);
}

.panel-browser--full {
  flex: 1 1 auto;
  width: auto !important;
  max-width: none;
}

.panel-browser {
  width: var(--sidebar-width);
  min-width: var(--sidebar-min-width);
  max-width: var(--sidebar-max-width);
}

.panel-timeline {
  flex: 1 1 auto;
  background: var(--color-bg-app);
  min-width: 320px;
}

.workflow-authoring-page--dragging {
  cursor: col-resize;
}

:deep(.workflow-panel-header) {
  display: none;
}

:deep(.workflow-panel-body) {
  flex: 1;
  overflow: hidden;
  padding: 0;
}

:deep(.workflow-name) {
  white-space: normal !important;
  overflow: visible !important;
  text-overflow: clip !important;
  word-break: break-word;
}
</style>
