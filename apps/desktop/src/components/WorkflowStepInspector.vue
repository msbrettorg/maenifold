<!-- audit-ok: max-loc — Inspector is the workflow step editing surface; consolidating field rendering, template editing, and tool config keeps related concerns together. Splitting would require prop-drilling shared state across 4+ subcomponents. -->
<script setup lang="ts">
import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Textarea } from '@/components/ui/textarea'
import { useWorkflowLibraryStore } from '../stores/workflow-library'
import type { ReasoningEffort, WorkflowStepDefinition, ToolHintPriority } from '../lib/workflow-asset-types'
import WorkflowMetadataForm from './WorkflowMetadataForm.vue'

const props = defineProps<{
  selectedStepId: string | null
}>()

const store = useWorkflowLibraryStore()
const { currentDefinition, draftDefinition } = storeToRefs(store)

const displayedDefinition = computed(() => draftDefinition.value ?? currentDefinition.value)
const selectedStep = computed<WorkflowStepDefinition | null>(() => {
  if (!props.selectedStepId || !displayedDefinition.value) return null
  return (
    displayedDefinition.value.steps.find(
      (step: { id: string }) => step.id === props.selectedStepId,
    ) ?? null
  )
})
const reasoningLevels: ReasoningEffort[] = ['low', 'medium', 'high']

const stepName = computed({
  get: () => selectedStep.value?.name ?? '',
  set: (value: string) => {
    if (selectedStep.value) store.updateStepName(selectedStep.value.id, value)
  },
})

const stepDescription = computed({
  get: () => selectedStep.value?.description ?? '',
  set: (value: string) => {
    if (selectedStep.value) store.updateStepDescription(selectedStep.value.id, value)
  },
})

const selectedEffort = computed<ReasoningEffort>({
  get: () => selectedStep.value?.metadata?.reasoning_effort ?? 'medium',
  set: (value) => {
    if (selectedStep.value) store.setStepReasoningEffort(selectedStep.value.id, value)
  },
})

const toolHintEntries = computed(() => {
  const hints = selectedStep.value?.metadata?.toolHints
  if (!hints) return []
  return Object.entries(hints).map(([name, hint]) => ({ name, ...hint }))
})

function formatReasoningLevel(level: ReasoningEffort): string {
  return level.charAt(0).toUpperCase() + level.slice(1)
}

function priorityClass(priority: ToolHintPriority): string {
  switch (priority) {
    case 'mandatory':
      return 'priority-mandatory'
    case 'recommended':
      return 'priority-recommended'
    case 'optional':
      return 'priority-optional'
    default:
      return ''
  }
}
</script>

<template>
  <aside class="h-full bg-[var(--color-bg-elevated)] overflow-y-auto flex flex-col pb-4" aria-label="Step inspector">
    <div v-if="!displayedDefinition" class="flex-1" aria-hidden="true" />

    <template v-else-if="selectedStep">
      <h2 class="panel-header min-h-11 m-0 mb-4 px-4 flex items-center gap-2 shrink-0">
        <span class="w-1.5 h-1.5 rounded-[var(--radius)] bg-[var(--color-node-workflow)]" />
        <span class="[font-weight:var(--weight-title)] text-[length:var(--text-tiny)] leading-none [font-family:var(--font-code)] tracking-[0.06em] text-[var(--color-text-secondary)]">Step inspector</span>
      </h2>

      <section class="inspector-section" aria-labelledby="inspector-section-prompt">
        <h3 id="inspector-section-prompt" class="section-label">Prompt</h3>
        <div class="form-group mx-4 flex flex-col gap-1">
          <Label class="field-label [font-weight:var(--weight-display)] text-[length:var(--text-tiny)] leading-[1.4] [font-family:var(--font-code)] text-[var(--color-text-secondary)] tracking-[0.05em]">Step name</Label>
          <Input v-model="stepName" type="text" />
        </div>
        <div class="form-group mx-4 mt-5 flex flex-col gap-1">
          <Label class="field-label [font-weight:var(--weight-display)] text-[length:var(--text-tiny)] leading-[1.4] [font-family:var(--font-code)] text-[var(--color-text-secondary)] tracking-[0.05em]">Description</Label>
          <Textarea v-model="stepDescription" rows="4" />
        </div>
      </section>

      <section class="inspector-section" aria-labelledby="inspector-section-tools">
        <h3 id="inspector-section-tools" class="section-label">Tools &amp; permissions</h3>
        <div v-if="toolHintEntries.length > 0" class="form-group mx-4 flex flex-col gap-1">
          <div class="flex flex-wrap gap-2">
            <span
              v-for="entry in toolHintEntries"
              :key="entry.name"
              class="tool-chip inline-flex items-center gap-1 py-1 px-2 rounded-[var(--radius-sm)] [font-weight:var(--weight-code)] text-[length:var(--text-tiny)] leading-[1.4] [font-family:var(--font-code)] text-[var(--color-text-primary)]"
              :title="entry.reason"
            >
              {{ entry.name }}
              <span class="priority-badge py-0.5 px-1 rounded [font-weight:var(--weight-title)] text-[length:var(--text-2xs)] leading-[1.4] [font-family:var(--font-code)] tracking-[0.03em]" :class="priorityClass(entry.priority)">
                {{ entry.priority }}
              </span>
            </span>
          </div>
        </div>
        <p v-else class="section-empty">No tool hints declared for this step.</p>
      </section>

      <section class="inspector-section" aria-labelledby="inspector-section-thinking">
        <h3 id="inspector-section-thinking" class="section-label">Sequential thinking</h3>
        <div class="form-group mx-4 flex flex-col gap-1">
          <Button
            variant="ghost"
            class="thinking-card flex items-center justify-between gap-3 p-3 rounded-[var(--radius-md)] cursor-pointer transition-[background-color,box-shadow] duration-[var(--transition-fast)]"
            :class="{ enabled: selectedStep.requiresSequentialThinking }"
            type="button"
            @click="store.setStepRequiresSequentialThinking(selectedStep.id, !selectedStep.requiresSequentialThinking)"
          >
            <div class="thinking-text flex flex-col gap-0.5 text-left">
              <span class="thinking-title [font-weight:var(--weight-body)] text-[length:var(--text-caption)] leading-[1.5] [font-family:var(--font-body)] text-[var(--color-text-primary)]">Sequential thinking</span>
              <span class="thinking-subtitle [font-weight:var(--weight-display)] text-[length:var(--text-tiny)] leading-[1.4] [font-family:var(--font-code)] text-[var(--color-text-secondary)] tracking-[0.05em]">Enable chain-of-thought</span>
            </div>
            <span class="thinking-indicator w-9 h-5 rounded flex items-center p-0.5 bg-[var(--color-border-subtle)] transition-[background] duration-[var(--transition-fast)]" :class="{ on: selectedStep.requiresSequentialThinking }">
              <span class="indicator-dot w-4 h-4 rounded-[var(--radius)] bg-[var(--color-text-primary)]" />
            </span>
          </Button>
        </div>
        <div class="form-group mx-4 mt-5 flex flex-col gap-1">
          <Label class="field-label [font-weight:var(--weight-display)] text-[length:var(--text-tiny)] leading-[1.4] [font-family:var(--font-code)] text-[var(--color-text-secondary)] tracking-[0.05em]">Reasoning effort</Label>
          <div class="effort-segments grid grid-cols-3 rounded-[var(--radius-md)] overflow-hidden" role="group" aria-label="Reasoning effort">
            <Button
              v-for="level in reasoningLevels"
              :key="level"
              variant="ghost"
              :class="['effort-segment text-center py-2 px-1 bg-[var(--color-border-subtle)] text-[var(--color-text-secondary)] border-0 [font-weight:var(--weight-title)] text-[length:var(--text-tiny)] leading-[1.4] [font-family:var(--font-code)] tracking-[0.05em] cursor-pointer', { active: selectedEffort === level }]"
              type="button"
              @click="selectedEffort = level"
            >
              {{ formatReasoningLevel(level) }}
            </Button>
          </div>
        </div>
      </section>

      <section class="inspector-section inspector-section--last" aria-labelledby="inspector-section-meta">
        <h3 id="inspector-section-meta" class="section-label">Meta</h3>
        <div class="form-group mx-4 flex flex-col gap-1">
          <Label class="field-label [font-weight:var(--weight-display)] text-[length:var(--text-tiny)] leading-[1.4] [font-family:var(--font-code)] text-[var(--color-text-secondary)] tracking-[0.05em]">Step ID</Label>
          <code class="[font-family:var(--font-code)] text-[length:var(--text-tiny)] text-[var(--color-text-secondary)]">{{ selectedStep.id }}</code>
        </div>
      </section>
    </template>

    <template v-else>
      <h2 class="panel-header min-h-11 m-0 mb-4 px-4 flex items-center gap-2 shrink-0">
        <span class="w-1.5 h-1.5 rounded-[var(--radius)] bg-[var(--color-node-workflow)]" />
        <span class="[font-weight:var(--weight-title)] text-[length:var(--text-tiny)] leading-none [font-family:var(--font-code)] tracking-[0.06em] text-[var(--color-text-secondary)]">Workflow</span>
      </h2>
      <div class="mx-4 flex flex-col gap-5">
        <WorkflowMetadataForm />
      </div>
      <div class="inspector-empty-state" role="status">
        <span class="empty-title">Select a step</span>
        <span class="empty-text">Pick a step from the timeline to edit its prompt, tools, and reasoning settings.</span>
      </div>
    </template>
  </aside>
</template>

<!-- audit-ok: dynamic-binding (color-mix throughout, inset box-shadows, dynamic state classes) -->
<style scoped>
.panel-header {
  background: color-mix(in srgb, var(--color-bg-elevated) 30%, var(--color-bg-app));
}

.tool-chip {
  background: color-mix(in srgb, var(--color-bg-elevated) 80%, var(--color-bg-app));
  box-shadow: inset 0 0 0 0.75px color-mix(in srgb, var(--color-border-subtle) 75%, transparent);
}

.priority-mandatory {
  background: color-mix(in srgb, var(--color-status-abandoned) 25%, transparent);
  color: var(--color-status-abandoned);
}

.priority-recommended {
  background: color-mix(in srgb, var(--color-node-workflow) 20%, transparent);
  color: var(--color-node-workflow);
}

.priority-optional {
  background: color-mix(in srgb, var(--color-text-secondary) 20%, transparent);
  color: var(--color-text-secondary);
}

.thinking-card {
  background: color-mix(in srgb, var(--color-bg-elevated) 86%, var(--color-bg-app));
  box-shadow: inset 0 0 0 0.75px color-mix(in srgb, var(--color-border-subtle) 65%, transparent);
}

.thinking-card.enabled {
  background: color-mix(in srgb, var(--color-node-workflow) 10%, var(--color-bg-elevated));
  box-shadow:
    inset 0 0 0 0.75px color-mix(in srgb, var(--color-node-workflow) 45%, var(--color-border-subtle)),
    0 0 0 2px color-mix(in srgb, var(--color-node-workflow) 12%, transparent);
}

.thinking-indicator.on {
  background: var(--color-node-workflow);
  justify-content: flex-end;
}

.effort-segments {
  background: color-mix(in srgb, var(--color-border-subtle) 75%, var(--color-bg-elevated));
  box-shadow: inset 0 0 0 0.75px color-mix(in srgb, var(--color-border-subtle) 65%, transparent);
}

.effort-segment + .effort-segment {
  box-shadow: inset 0.75px 0 0 color-mix(in srgb, var(--color-border-subtle) 75%, transparent);
}

.effort-segment.active {
  background: color-mix(in srgb, var(--color-node-workflow) 16%, var(--color-border-subtle));
  color: var(--color-node-workflow);
}
</style>
