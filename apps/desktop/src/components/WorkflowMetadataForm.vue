<script setup lang="ts">
import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Textarea } from '@/components/ui/textarea'
import type { WorkflowCategory } from '../lib/workflow-asset-types'
import { useWorkflowLibraryStore } from '../stores/workflow-library'

const CATEGORY_OPTIONS: WorkflowCategory[] = [
  'reasoning',
  'scientific',
  'multi-agent',
  'devops',
  'memory',
  'custom',
]

const store = useWorkflowLibraryStore()
const { currentDefinition, draftDefinition } = storeToRefs(store)

const workflow = computed(() => draftDefinition.value ?? currentDefinition.value)
const workflowName = computed({
  get: () => workflow.value?.name ?? '',
  set: (value: string) => store.updateWorkflowName(value),
})
const workflowEmoji = computed({
  get: () => workflow.value?.emoji ?? '',
  set: (value: string) => store.updateWorkflowEmoji(value),
})
const workflowDescription = computed({
  get: () => workflow.value?.description ?? '',
  set: (value: string) => store.updateWorkflowDescription(value),
})
const workflowTriggers = computed({
  get: () => (workflow.value?.triggers ?? []).join(', '),
  set: (value: string) =>
    store.updateWorkflowTriggers(
      value
        .split(',')
        .map((entry) => entry.trim())
        .filter(Boolean),
    ),
})
const workflowCategory = computed({
  get: () => workflow.value?.category ?? '',
  set: (value: string) =>
    store.updateWorkflowCategory(value ? (value as WorkflowCategory) : null),
})
</script>

<template>
  <div v-if="workflow" class="flex flex-col gap-5" role="form" aria-label="Workflow metadata">
    <div class="flex flex-col gap-1">
      <Label class="[font-weight:var(--weight-display)] text-[length:var(--text-tiny)] leading-[1.4] [font-family:var(--font-code)] text-[var(--color-text-secondary)] tracking-[0.05em]">Workflow ID</Label>
      <code class="[font-family:var(--font-code)] text-[length:var(--text-caption)] text-[var(--color-text-secondary)] py-2 leading-[1.5]">{{ workflow.id }}</code>
    </div>

    <div class="flex flex-col gap-1">
      <Label class="[font-weight:var(--weight-display)] text-[length:var(--text-tiny)] leading-[1.4] [font-family:var(--font-code)] text-[var(--color-text-secondary)] tracking-[0.05em]" for="workflow-name">Name</Label>
      <Input id="workflow-name" v-model="workflowName" type="text" />
    </div>

    <div class="grid grid-cols-2 gap-3">
      <div class="flex flex-col gap-1">
        <Label class="[font-weight:var(--weight-display)] text-[length:var(--text-tiny)] leading-[1.4] [font-family:var(--font-code)] text-[var(--color-text-secondary)] tracking-[0.05em]" for="workflow-emoji">Emoji</Label>
        <Input id="workflow-emoji" v-model="workflowEmoji" type="text" />
      </div>

      <div class="flex flex-col gap-1">
        <Label class="[font-weight:var(--weight-display)] text-[length:var(--text-tiny)] leading-[1.4] [font-family:var(--font-code)] text-[var(--color-text-secondary)] tracking-[0.05em]">Category</Label>
        <Select :model-value="workflowCategory" @update:model-value="(v) => store.updateWorkflowCategory(v ? (String(v) as WorkflowCategory) : null)">
          <SelectTrigger>
            <SelectValue placeholder="Auto" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem v-for="option in CATEGORY_OPTIONS" :key="option" :value="option">
              {{ option }}
            </SelectItem>
          </SelectContent>
        </Select>
      </div>
    </div>

    <div class="flex flex-col gap-1">
      <Label class="[font-weight:var(--weight-display)] text-[length:var(--text-tiny)] leading-[1.4] [font-family:var(--font-code)] text-[var(--color-text-secondary)] tracking-[0.05em]" for="workflow-description">Description</Label>
      <Textarea
        id="workflow-description"
        v-model="workflowDescription"
        rows="4"
      />
    </div>

    <div class="flex flex-col gap-1">
      <Label class="[font-weight:var(--weight-display)] text-[length:var(--text-tiny)] leading-[1.4] [font-family:var(--font-code)] text-[var(--color-text-secondary)] tracking-[0.05em]" for="workflow-triggers">Triggers</Label>
      <Input
        id="workflow-triggers"
        v-model="workflowTriggers"
        placeholder="research, plan, investigate"
        type="text"
      />
    </div>

    <div class="flex flex-col gap-1">
      <Label class="[font-weight:var(--weight-display)] text-[length:var(--text-tiny)] leading-[1.4] [font-family:var(--font-code)] text-[var(--color-text-secondary)] tracking-[0.05em]">Sequential thinking</Label>
      <!-- audit-ok: dynamic-binding — color-mix() border + box-shadow only when toggle is enabled -->
      <Button
        variant="ghost"
        class="flex items-center justify-between gap-3 px-4 py-3 bg-[var(--color-bg-elevated)] rounded-[var(--radius-md)] border border-[var(--color-border-subtle)] text-[var(--color-text-primary)] cursor-pointer text-left"
        :style="workflow.sequentialThinkingEnabled ? { borderColor: 'color-mix(in srgb, var(--color-node-workflow) 30%, var(--color-border-subtle))', boxShadow: '0 0 0 1px color-mix(in srgb, var(--color-node-workflow) 16%, transparent)' } : {}"
        type="button"
        @click="store.updateWorkflowSequentialThinkingEnabled(!workflow.sequentialThinkingEnabled)"
      >
        <span class="[font-weight:var(--weight-body)] text-[length:var(--text-caption)] leading-[1.5] [font-family:var(--font-body)]">Enable workflow-level sequential thinking</span>
        <span
          class="w-9 h-5 rounded-[4px] flex items-center p-0.5 transition-[background] duration-[var(--transition-fast)]"
          :class="workflow.sequentialThinkingEnabled ? 'bg-[var(--color-node-workflow)] justify-end' : 'bg-[var(--color-border-subtle)]'"
        >
          <span class="w-4 h-4 rounded-[var(--radius)] bg-[var(--color-text-primary)]" />
        </span>
      </Button>
    </div>

    <div class="flex flex-col gap-1">
      <Label class="[font-weight:var(--weight-display)] text-[length:var(--text-tiny)] leading-[1.4] [font-family:var(--font-code)] text-[var(--color-text-secondary)] tracking-[0.05em]">Multi-agent</Label>
      <!-- audit-ok: dynamic-binding — color-mix() border + box-shadow only when toggle is enabled -->
      <Button
        variant="ghost"
        class="flex items-center justify-between gap-3 px-4 py-3 bg-[var(--color-bg-elevated)] rounded-[var(--radius-md)] border border-[var(--color-border-subtle)] text-[var(--color-text-primary)] cursor-pointer text-left"
        :style="workflow.multiAgent ? { borderColor: 'color-mix(in srgb, var(--color-node-workflow) 30%, var(--color-border-subtle))', boxShadow: '0 0 0 1px color-mix(in srgb, var(--color-node-workflow) 16%, transparent)' } : {}"
        type="button"
        @click="store.updateWorkflowMultiAgent(!workflow.multiAgent)"
      >
        <span class="[font-weight:var(--weight-body)] text-[length:var(--text-caption)] leading-[1.5] [font-family:var(--font-body)]">Mark this workflow as multi-agent</span>
        <span
          class="w-9 h-5 rounded-[4px] flex items-center p-0.5 transition-[background] duration-[var(--transition-fast)]"
          :class="workflow.multiAgent ? 'bg-[var(--color-node-workflow)] justify-end' : 'bg-[var(--color-border-subtle)]'"
        >
          <span class="w-4 h-4 rounded-[var(--radius)] bg-[var(--color-text-primary)]" />
        </span>
      </Button>
    </div>
  </div>
</template>
