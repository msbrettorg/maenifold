<script setup lang="ts">
import { ref } from 'vue'
import {
  Brain,
  FlaskConical,
  Users,
  Cog,
  BookOpen,
  Lightbulb,
  AlertTriangle,
  Copy,
  Trash2,
  type LucideIcon,
} from 'lucide-vue-next'
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog'
import { Button } from '@/components/ui/button'
import { categorizeWorkflow, type WorkflowAssetSummary, type WorkflowCategory } from '../lib/workflow-asset-types'

defineProps<{
  currentId: string | null
  filteredWorkflows: WorkflowAssetSummary[]
  isDirty: boolean
  stepCounts: Record<string, number>
}>()

const emit = defineEmits<{
  (e: 'select', id: string): void
  (e: 'duplicate', id: string): void
  (e: 'delete', id: string): void
}>()

const deleteWorkflow = ref<WorkflowAssetSummary | null>(null)
const deleteWorkflowId = ref<string | null>(null)

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

function openDeleteDialog(workflow: WorkflowAssetSummary) {
  deleteWorkflow.value = workflow
  deleteWorkflowId.value = workflow.id
}

function closeDeleteDialog(open: boolean) {
  if (!open) {
    deleteWorkflow.value = null
  }
}

function confirmDelete() {
  if (!deleteWorkflowId.value) return
  emit('delete', deleteWorkflowId.value)
  deleteWorkflow.value = null
  deleteWorkflowId.value = null
}

function cancelDelete() {
  deleteWorkflow.value = null
  deleteWorkflowId.value = null
}
</script>

<template>
  <ul
    class="m-0 flex list-none flex-col gap-1 px-2 pb-4"
    aria-label="Available workflows"
  >
    <li
      v-for="wf in filteredWorkflows"
      :key="wf.id"
      class="group flex items-stretch rounded-lg border-l-[3px] bg-bg-surface transition-colors duration-150 hover:bg-bg-surface-hover"
      :class="wf.id === currentId
        ? 'border-l-brand bg-bg-surface-hover'
        : 'border-l-transparent'"
    >
      <Button
        type="button"
        variant="ghost"
        class="flex h-auto w-full items-center justify-start gap-2 rounded-lg bg-transparent p-2 text-left font-normal hover:bg-transparent"
        :data-workflow-id="wf.id"
        :aria-current="wf.id === currentId ? 'page' : undefined"
        @click="emit('select', wf.id)"
      >
        <component
          :is="iconForWorkflow(wf)"
          class="size-4 shrink-0 text-text-secondary"
          :stroke-width="1.5"
          aria-hidden="true"
        />
        <div class="flex min-w-0 flex-1 flex-col gap-px">
          <span
            class="truncate text-sm font-medium leading-snug text-text-primary"
            :title="humanizeName(wf.name, wf.id)"
          >
            {{ humanizeName(wf.name, wf.id) }}
          </span>
          <div class="flex flex-wrap items-center gap-1.5">
            <span class="text-xs leading-snug text-text-secondary">
              {{ categoryLabel(categorizeWorkflow(wf)) }}
            </span>
            <span
              v-if="stepLabel(stepCounts[wf.id])"
              class="text-xs leading-snug text-text-tertiary"
            >
              {{ stepLabel(stepCounts[wf.id]) }}
            </span>
          </div>
        </div>
        <span
          v-if="wf.id === currentId && isDirty"
          class="ml-2 size-1.5 shrink-0 self-center rounded-full bg-brand"
          aria-label="Unsaved changes"
        />
      </Button>
      <div class="flex shrink-0 items-center gap-1 pr-2 opacity-100 transition-opacity duration-150 sm:opacity-0 sm:group-hover:opacity-100 sm:focus-within:opacity-100">
        <Button
          type="button"
          variant="ghost"
          size="icon"
          class="size-8 text-text-secondary hover:text-text-primary"
          :aria-label="`Duplicate ${humanizeName(wf.name, wf.id)}`"
          title="Duplicate workflow"
          @click.stop="emit('duplicate', wf.id)"
        >
          <Copy class="size-4" aria-hidden="true" />
        </Button>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          class="size-8 text-text-secondary hover:text-danger"
          :aria-label="`Delete ${humanizeName(wf.name, wf.id)}`"
          title="Delete workflow"
          @click.stop="openDeleteDialog(wf)"
        >
          <Trash2 class="size-4" aria-hidden="true" />
        </Button>
      </div>
    </li>

    <li
      v-if="filteredWorkflows.length === 0"
      class="flex flex-col items-center gap-2 px-4 py-8 text-center"
    >
      <AlertTriangle
        class="size-5 text-text-tertiary"
        :stroke-width="1.5"
        aria-hidden="true"
      />
      <span class="text-sm font-medium leading-relaxed text-text-secondary">
        No matching workflows
      </span>
      <span class="text-xs leading-relaxed text-text-tertiary">
        Try a different search or filter
      </span>
    </li>
  </ul>

  <AlertDialog
    :open="deleteWorkflow !== null"
    @update:open="closeDeleteDialog"
  >
    <AlertDialogContent class="sm:max-w-[28rem]">
      <AlertDialogHeader>
        <AlertDialogTitle>
          Delete workflow {{ deleteWorkflow ? humanizeName(deleteWorkflow.name, deleteWorkflow.id) : '' }}?
        </AlertDialogTitle>
        <AlertDialogDescription>
          This will permanently delete the workflow asset. Pipelines that reference this workflow will fail at next run.
        </AlertDialogDescription>
      </AlertDialogHeader>
      <AlertDialogFooter>
        <AlertDialogCancel @click="cancelDelete">Cancel</AlertDialogCancel>
        <AlertDialogAction
          variant="destructive"
          @click="confirmDelete"
        >
          Delete
        </AlertDialogAction>
      </AlertDialogFooter>
    </AlertDialogContent>
  </AlertDialog>
</template>
