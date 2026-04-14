<script setup lang="ts">
import type { WorkflowAssetSummary } from '../lib/workflow-asset-types'

defineProps<{
  currentId: string | null
  filteredWorkflows: WorkflowAssetSummary[]
  isDirty: boolean
}>()

const emit = defineEmits<{
  (e: 'select', id: string): void
}>()
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
        <span class="workflow-name">{{ wf.name }}</span>
        <div class="workflow-badges">
          <span v-if="wf.multiAgent" class="badge-multi-agent">MULTI-AGENT</span>
        </div>
      </div>
    </li>

    <li v-if="filteredWorkflows.length === 0" class="empty-state">No workflows found</li>
  </ul>
</template>

<style scoped>
.workflow-list { flex: 1; list-style: none; margin: 0; padding: 0 var(--space-3) var(--space-4); overflow-y: auto; display: flex; flex-direction: column; gap: var(--space-2); scrollbar-width: thin; scrollbar-color: var(--card) transparent; }
.workflow-item { display: flex; align-items: flex-start; gap: var(--space-3); padding: var(--space-3); border: 1px solid var(--border); border-left: 4px solid transparent; border-radius: var(--radius-md); background: var(--card); cursor: pointer; transition: background var(--transition-fast), border-color var(--transition-fast), box-shadow var(--transition-fast); }
.workflow-item:hover { background: var(--muted); }
.workflow-item--selected { background: var(--muted); border-left-color: var(--color-node-workflow); border-color: color-mix(in srgb, var(--color-node-workflow) 30%, var(--border)); box-shadow: 0 0 0 1px color-mix(in srgb, var(--color-node-workflow) 18%, transparent); }
.workflow-item--dirty::before { content: ''; align-self: center; width: 4px; height: 4px; border-radius: 50%; background: var(--color-status-active); margin-right: calc(var(--space-1) * -1); }
.workflow-emoji { font-size: var(--text-body-sm); line-height: 1.25; flex-shrink: 0; }
.workflow-info { display: flex; flex-direction: column; gap: var(--space-1); min-width: 0; }
.workflow-name { font: var(--weight-title) var(--text-caption)/1.4 var(--font-body); color: var(--foreground); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.workflow-badges { display: flex; gap: var(--space-1); flex-wrap: wrap; }
.badge-multi-agent { display: inline-flex; align-items: center; padding: 1px var(--space-1); border-radius: var(--radius-sm); background: color-mix(in srgb, var(--color-node-workflow) 12%, var(--card)); color: var(--color-node-workflow); font: var(--weight-title) var(--text-2xs)/1.5 var(--font-code); letter-spacing: 0.03em; text-transform: uppercase; }
.empty-state { padding: var(--space-8) var(--space-4); display: flex; flex-direction: column; align-items: center; gap: var(--space-2); text-align: center; color: transparent; font-size: 0; }
.empty-state::before { content: '⌕'; font-size: var(--text-headline); line-height: 1; color: color-mix(in srgb, var(--muted-foreground) 70%, transparent); }
.empty-state::after { content: 'Nothing found\A Try a different search'; white-space: pre-line; font-family: var(--font-body); font-size: var(--text-caption); line-height: 1.6; color: var(--muted-foreground); }
</style>
