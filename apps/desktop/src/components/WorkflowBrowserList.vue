<script setup lang="ts">
import { categorizeWorkflow, type WorkflowAssetSummary, type WorkflowCategory } from '../lib/workflow-asset-types'
import { Brain, FlaskConical, Users, Cog, BookOpen, Lightbulb, AlertTriangle, type LucideIcon } from 'lucide-vue-next'

defineProps<{
  currentId: string | null
  filteredWorkflows: WorkflowAssetSummary[]
  isDirty: boolean
  stepCounts: Record<string, number>
}>()

const emit = defineEmits<{
  (e: 'select', id: string): void
}>()

const categoryIconMap: Record<WorkflowCategory, LucideIcon> = {
  reasoning: Brain,
  scientific: FlaskConical,
  'multi-agent': Users,
  devops: Cog,
  memory: BookOpen,
  custom: Lightbulb,
}

function iconForWorkflow(wf: WorkflowAssetSummary): LucideIcon {
  return categoryIconMap[categorizeWorkflow(wf)] ?? Brain
}

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
      <component :is="iconForWorkflow(wf)" class="size-4 shrink-0 text-text-secondary" :stroke-width="1.5" />
      <div class="workflow-info">
        <span class="workflow-name">{{ humanizeName(wf.name, wf.id) }}</span>
        <div class="workflow-meta">
          <span class="meta-category">{{ categoryLabel(categorizeWorkflow(wf)) }}</span>
          <span v-if="stepLabel(stepCounts[wf.id])" class="meta-steps">{{ stepLabel(stepCounts[wf.id]) }}</span>
        </div>
      </div>
    </li>

    <li v-if="filteredWorkflows.length === 0" class="empty-state">
      <AlertTriangle class="size-5 text-text-tertiary" :stroke-width="1.5" />
      <span class="empty-message">No matching workflows</span>
      <span class="empty-hint">Try a different search or filter</span>
    </li>
  </ul>
</template>

<style scoped>
.workflow-list { flex: 1; list-style: none; margin: 0; padding: 0 8px 16px; overflow-y: auto; display: flex; flex-direction: column; gap: 4px; scrollbar-width: thin; scrollbar-color: var(--color-bg-elevated) transparent; }
.workflow-item { display: flex; align-items: center; gap: 8px; padding: 8px; border-left: 3px solid transparent; border-radius: 8px; background: var(--color-bg-surface); cursor: pointer; transition: background 0.15s; }
.workflow-item:hover { background: var(--color-bg-surface-hover); }
.workflow-item--selected { background: var(--color-bg-surface-hover); border-left-color: var(--color-brand); }
.workflow-item--dirty::before { content: ''; align-self: center; width: 4px; height: 4px; border-radius: 50%; background: var(--color-brand); margin-right: -4px; }
.workflow-info { display: flex; flex-direction: column; gap: 1px; min-width: 0; }
.workflow-name { font-size: 13px; font-weight: 500; line-height: 1.4; color: var(--color-text-primary); white-space: normal; word-break: break-word; }
.workflow-meta { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }
.meta-category { font-size: 11px; line-height: 1.4; color: var(--color-text-secondary); }
.meta-steps { font-size: 11px; line-height: 1.4; font-family: var(--font-body); color: var(--color-text-tertiary); }
.empty-state { display: flex; flex-direction: column; align-items: center; gap: 8px; padding: 32px 16px; text-align: center; }
.empty-message { font-size: 13px; font-weight: 500; line-height: 1.6; color: var(--color-text-secondary); }
.empty-hint { font-size: 11px; line-height: 1.6; color: var(--color-text-tertiary); }
</style>
