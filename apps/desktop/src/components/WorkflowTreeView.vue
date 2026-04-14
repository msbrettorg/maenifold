<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useWorkflowTree } from '../composables/useWorkflowTree'
import { WorkflowTreeLevel } from './WorkflowTreeLevel'
import WorkflowTreeErrorState from './WorkflowTreeErrorState.vue'
import WorkflowTreeLoadingState from './WorkflowTreeLoadingState.vue'

const props = defineProps<{ uri: string }>()

const { loadTree, tree, loading, error } = useWorkflowTree()
const isRootLoaded = computed(() => tree.value !== null && tree.value.type === 'workflow')

onMounted(() => {
  void loadTree(props.uri)
})

function retry() {
  void loadTree(props.uri)
}
</script>

<template>
  <div class="workflow-tree-view">
    <WorkflowTreeLoadingState v-if="loading" />
    <WorkflowTreeErrorState v-else-if="error" :error="error" @retry="retry" />
    <div v-else-if="isRootLoaded && tree" class="tree-root-scroll">
      <WorkflowTreeLevel :node="tree" :depth="0" />
    </div>
    <div v-else class="tree-empty">No workflow data found.</div>
  </div>
</template>

<style scoped>
.workflow-tree-view {
  background: var(--bg-code);
  min-height: 200px;
  padding: 1.5rem;
  border-radius: 12px;
  font-family: var(--font-body);
}
.tree-root-scroll {
  overflow-x: auto;
  overflow-y: visible;
  padding-bottom: 1rem;
  scrollbar-width: thin;
  scrollbar-color: color-mix(in srgb, var(--border-default) 72%, var(--bg-code)) transparent;
}
.tree-root-scroll::-webkit-scrollbar { height: 6px; }
.tree-root-scroll::-webkit-scrollbar-track { background: transparent; }
.tree-root-scroll::-webkit-scrollbar-thumb {
  background: color-mix(in srgb, var(--border-default) 72%, var(--bg-code));
  border-radius: 3px;
}
:global(.tree-level) {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0;
  flex-shrink: 0;
}
:global(.children-row) {
  display: flex;
  flex-direction: row;
  align-items: flex-start;
  gap: 24px;
  flex-shrink: 0;
}
:global(.connector-wrapper) {
  display: flex;
  justify-content: center;
  align-items: flex-start;
  width: 100%;
  overflow: visible;
}
:global(.tree-connector) {
  display: block;
  overflow: visible;
  flex-shrink: 0;
}
.tree-empty {
  text-align: center;
  color: var(--text-secondary);
  font-size: var(--text-caption);
  padding: 2rem 0;
}
</style>
