<script setup lang="ts">
import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Switch } from '@/components/ui/switch'
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

const labelClass =
  'font-mono text-xs font-semibold text-text-secondary tracking-wider uppercase'
</script>

<template>
  <div v-if="workflow" class="flex flex-col gap-5" role="form" aria-label="Workflow metadata">
    <div class="flex flex-col gap-1">
      <Label :class="labelClass">Workflow ID</Label>
      <code class="font-mono text-sm text-text-secondary py-2 leading-relaxed">{{ workflow.id }}</code>
    </div>

    <div class="flex flex-col gap-1">
      <Label :class="labelClass" for="workflow-name">Name</Label>
      <Input id="workflow-name" v-model="workflowName" type="text" />
    </div>

    <div class="grid grid-cols-2 gap-3">
      <div class="flex flex-col gap-1">
        <Label :class="labelClass" for="workflow-emoji">Emoji</Label>
        <Input id="workflow-emoji" v-model="workflowEmoji" type="text" />
      </div>

      <div class="flex flex-col gap-1">
        <Label :class="labelClass">Category</Label>
        <Select
          :model-value="workflowCategory"
          @update:model-value="(v) => store.updateWorkflowCategory(v ? (String(v) as WorkflowCategory) : null)"
        >
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
      <Label :class="labelClass" for="workflow-description">Description</Label>
      <Textarea id="workflow-description" v-model="workflowDescription" rows="4" />
    </div>

    <div class="flex flex-col gap-1">
      <Label :class="labelClass" for="workflow-triggers">Triggers</Label>
      <Input
        id="workflow-triggers"
        v-model="workflowTriggers"
        placeholder="research, plan, investigate"
        type="text"
      />
    </div>

    <div class="flex flex-col gap-1">
      <Label :class="labelClass">Sequential thinking</Label>
      <div
        class="flex items-center justify-between gap-3 px-4 py-3 bg-surface-highest rounded-md border border-border-subtle"
        :class="workflow.sequentialThinkingEnabled ? 'border-accent-workflow ring-1 ring-accent-workflow/30' : ''"
      >
        <Label
          for="workflow-sequential-thinking"
          class="text-sm font-normal text-text-primary leading-relaxed cursor-pointer"
        >
          Enable workflow-level sequential thinking
        </Label>
        <Switch
          id="workflow-sequential-thinking"
          :model-value="workflow.sequentialThinkingEnabled"
          @update:model-value="(v) => store.updateWorkflowSequentialThinkingEnabled(Boolean(v))"
        />
      </div>
    </div>

    <div class="flex flex-col gap-1">
      <Label :class="labelClass">Multi-agent</Label>
      <div
        class="flex items-center justify-between gap-3 px-4 py-3 bg-surface-highest rounded-md border border-border-subtle"
        :class="workflow.multiAgent ? 'border-accent-workflow ring-1 ring-accent-workflow/30' : ''"
      >
        <Label
          for="workflow-multi-agent"
          class="text-sm font-normal text-text-primary leading-relaxed cursor-pointer"
        >
          Mark this workflow as multi-agent
        </Label>
        <Switch
          id="workflow-multi-agent"
          :model-value="workflow.multiAgent"
          @update:model-value="(v) => store.updateWorkflowMultiAgent(Boolean(v))"
        />
      </div>
    </div>
  </div>
</template>
