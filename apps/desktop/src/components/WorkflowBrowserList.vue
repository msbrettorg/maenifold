<script setup lang="ts">
import { categorizeWorkflow, type WorkflowAssetSummary, type WorkflowCategory } from '../lib/workflow-asset-types'

defineProps<{
  currentId: string | null
  filteredWorkflows: WorkflowAssetSummary[]
  isDirty: boolean
  stepCounts: Record<string, number>
}>()

const emit = defineEmits<{
  (e: 'select', id: string): void
}>()

function humanizeName(name: string, id: string): string {
  const value = name.trim() || id
  if (/^[a-z0-9]+(?:[-_][a-z0-9]+)+$/.test(value)) {
    return value.replace(/[-_]+/g, ' ').replace(/^./, (c) => c.toUpperCase())
  }
  return value
}

function categoryLabel(cat: WorkflowCategory): string {
  if (cat === 'multi-agent') return 'Multi-agent'
  if (cat === 'devops') return 'DevOps'
  return cat.charAt(0).toUpperCase() + cat.slice(1)
}

function stepLabel(count?: number): string | null {
  if (count === undefined) return null
  return `${count} ${count === 1 ? 'step' : 'steps'}`
}
</script>

<template>
  <ul class="workflow-list" aria-label="Available workflows">
    <li
      v-for="wf in filteredWorkflows"
      :key="wf.id"
      :class="[
        'workflow-item',
        {
          'workflow-item--dirty': wf.id === currentId && isDirty,
          'workflow-item--selected': wf.id === currentId,
        },
      ]"
      @click="emit('select', wf.id)"
    >
      <span class="workflow-emoji">{{ wf.emoji }}</span>
      <div class="workflow-info">
        <span class="workflow-name">{{ humanizeName(wf.name, wf.id) }}</span>
        <div class="workflow-meta">
          <span class="meta-category">{{ categoryLabel(categorizeWorkflow(wf)) }}</span>
          <span v-if="stepLabel(stepCounts[wf.id])" class="meta-steps">{{ stepLabel(stepCounts[wf.id]) }}</span>
        </div>
      </div>
    </li>

    <li v-if="filteredWorkflows.length === 0" class="empty-state">
      <span class="empty-icon">⌕</span>
      <span class="empty-message">No matching workflows</span>
      <span class="empty-hint">Try a different search or filter</span>
    </li>
  </ul>
</template>

<style scoped>
.workflow-list { flex: 1; list-style: none; margin: 0; padding: 0 var(--space-2) var(--space-4); overflow-y: auto; display: flex; flex-direction: column; gap: var(--space-1); scrollbar-width: thin; scrollbar-color: var(--card) transparent; }
.workflow-item { display: flex; align-items: flex-start; gap: var(--space-2); padding: var(--space-2); border-left: 4px solid transparent; border-radius: var(--radius-sm); background: #201f1f; cursor: pointer; transition: background var(--transition-fast); }
.workflow-item:hover { background: #2a2a2a; }
.workflow-item--selected { background: #2a2a2a; border-left-color: var(--color-node-workflow); }
.workflow-item--dirty::before { content: ''; align-self: center; width: 4px; height: 4px; border-radius: 50%; background: var(--color-status-active); margin-right: calc(var(--space-1) * -1); }
.workflow-emoji { font-size: var(--text-body-sm); line-height: 1.4; flex-shrink: 0; }
.workflow-info { display: flex; flex-direction: column; gap: 1px; min-width: 0; }
.workflow-name { font: var(--weight-title) var(--text-caption)/1.4 var(--font-body); color: var(--foreground); white-space: normal; overflow: visible; text-overflow: clip; word-break: break-word; }
.workflow-meta { display: flex; align-items: center; gap: var(--space-1); flex-wrap: wrap; }
.meta-category { display: inline-flex; align-items: center; min-height: 18px; padding: 0 var(--space-1); border-radius: var(--radius-sm); background: rgba(66, 71, 83, 0.15); font: var(--weight-body) var(--text-xs)/1.4 var(--font-body); color: var(--muted-foreground); }
.meta-steps { font: var(--weight-body) var(--text-tiny)/1.4 var(--font-code); color: var(--muted-foreground); }
.empty-state { display: flex; flex-direction: column; align-items: center; gap: var(--space-2); padding: var(--space-8) var(--space-4); text-align: center; }
.empty-icon { font-size: var(--text-headline); line-height: 1; color: color-mix(in srgb, var(--muted-foreground) 50%, transparent); }
.empty-message { font: var(--weight-title) var(--text-caption)/1.6 var(--font-body); color: var(--foreground); }
.empty-hint { font: var(--weight-body) var(--text-tiny)/1.6 var(--font-body); color: #707070; }
</style>
