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
  <ul class="flex-1 list-none m-0 p-0 px-2 pb-4 overflow-y-auto flex flex-col gap-1 [scrollbar-width:thin] [scrollbar-color:var(--color-bg-elevated)_transparent]" aria-label="Available workflows">
    <li
      v-for="wf in filteredWorkflows"
      :key="wf.id"
      :class="[
        'workflow-item flex items-center gap-2 p-2 border-l-[3px] border-l-transparent rounded-[8px] bg-[var(--color-bg-surface)] cursor-pointer transition-[background] duration-150',
        {
          'workflow-item--dirty': wf.id === currentId && isDirty,
          'workflow-item--selected bg-[var(--color-bg-surface-hover)] border-l-[var(--color-brand)]': wf.id === currentId,
        },
      ]"
      @click="emit('select', wf.id)"
    >
      <component :is="iconForWorkflow(wf)" class="size-4 shrink-0 text-text-secondary" :stroke-width="1.5" />
      <div class="flex flex-col gap-[1px] min-w-0">
        <span class="text-[13px] [font-weight:500] leading-[1.4] text-[var(--color-text-primary)] whitespace-normal break-words">{{ humanizeName(wf.name, wf.id) }}</span>
        <div class="flex items-center gap-[6px] flex-wrap">
          <span class="text-[11px] leading-[1.4] text-[var(--color-text-secondary)]">{{ categoryLabel(categorizeWorkflow(wf)) }}</span>
          <span v-if="stepLabel(stepCounts[wf.id])" class="text-[11px] leading-[1.4] [font-family:var(--font-body)] text-[var(--color-text-tertiary)]">{{ stepLabel(stepCounts[wf.id]) }}</span>
        </div>
      </div>
    </li>

    <li v-if="filteredWorkflows.length === 0" class="flex flex-col items-center gap-2 py-8 px-4 text-center">
      <AlertTriangle class="size-5 text-text-tertiary" :stroke-width="1.5" />
      <span class="text-[13px] [font-weight:500] leading-[1.6] text-[var(--color-text-secondary)]">No matching workflows</span>
      <span class="text-[11px] leading-[1.6] text-[var(--color-text-tertiary)]">Try a different search or filter</span>
    </li>
  </ul>
</template>

<!-- audit-ok: dynamic-binding (dirty indicator ::before pseudo-element with dynamic content) -->
<style scoped>
.workflow-item:hover { background: var(--color-bg-surface-hover); }
.workflow-item--dirty::before { content: ''; align-self: center; width: 4px; height: 4px; border-radius: 50%; background: var(--color-brand); margin-right: -4px; }
</style>
