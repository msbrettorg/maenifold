<script setup lang="ts">
import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Textarea } from '@/components/ui/textarea'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Switch } from '@/components/ui/switch'
import { Badge } from '@/components/ui/badge'
import { useWorkflowLibraryStore } from '../stores/workflow-library'
import type {
  ReasoningEffort,
  WorkflowStepDefinition,
  ToolHintPriority,
} from '../lib/workflow-asset-types'
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

const requiresThinking = computed<boolean>({
  get: () => selectedStep.value?.requiresSequentialThinking ?? false,
  set: (value: boolean) => {
    if (selectedStep.value) store.setStepRequiresSequentialThinking(selectedStep.value.id, value)
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

function priorityVariant(
  priority: ToolHintPriority,
): 'default' | 'secondary' | 'destructive' | 'outline' {
  switch (priority) {
    case 'mandatory':
      return 'destructive'
    case 'recommended':
      return 'default'
    case 'optional':
      return 'secondary'
    default:
      return 'outline'
  }
}
</script>

<template>
  <aside
    class="h-full flex flex-col bg-background"
    aria-label="Step inspector"
  >
    <ScrollArea class="flex-1">
    <div v-if="!displayedDefinition" class="flex-1" aria-hidden="true" />

    <template v-else-if="selectedStep">
      <div class="p-4 space-y-4">
        <div class="flex items-center gap-2">
          <span class="w-1.5 h-1.5 rounded-full bg-primary" />
          <span
            class="text-xs font-semibold tracking-wider uppercase text-muted-foreground"
          >
            Step inspector
          </span>
        </div>

        <Tabs default-value="prompt" class="w-full">
          <TabsList class="grid w-full grid-cols-4">
            <TabsTrigger value="prompt">Prompt</TabsTrigger>
            <TabsTrigger value="tools">Tools</TabsTrigger>
            <TabsTrigger value="thinking">Thinking</TabsTrigger>
            <TabsTrigger value="meta">Meta</TabsTrigger>
          </TabsList>

          <TabsContent value="prompt" class="mt-4">
            <Card>
              <CardHeader>
                <CardTitle class="text-sm">Prompt</CardTitle>
              </CardHeader>
              <CardContent class="space-y-4">
                <div class="flex flex-col gap-2">
                  <Label for="step-name" class="text-xs uppercase tracking-wider text-muted-foreground">
                    Step name
                  </Label>
                  <Input id="step-name" v-model="stepName" type="text" />
                </div>
                <div class="flex flex-col gap-2">
                  <Label
                    for="step-description"
                    class="text-xs uppercase tracking-wider text-muted-foreground"
                  >
                    Description
                  </Label>
                  <Textarea id="step-description" v-model="stepDescription" :rows="4" />
                </div>
              </CardContent>
            </Card>
          </TabsContent>

          <TabsContent value="tools" class="mt-4">
            <Card>
              <CardHeader>
                <CardTitle class="text-sm">Tools &amp; permissions</CardTitle>
              </CardHeader>
              <CardContent>
                <div v-if="toolHintEntries.length > 0" class="flex flex-wrap gap-2">
                  <Card
                    v-for="entry in toolHintEntries"
                    :key="entry.name"
                    class="bg-muted/40"
                    :title="entry.reason"
                  >
                    <CardContent class="flex items-center gap-1.5 px-2 py-1">
                      <span class="font-mono text-xs text-foreground">{{ entry.name }}</span>
                      <Badge :variant="priorityVariant(entry.priority)" class="text-[10px] uppercase">
                        {{ entry.priority }}
                      </Badge>
                    </CardContent>
                  </Card>
                </div>
                <p v-else class="text-sm text-muted-foreground">
                  No tool hints declared for this step.
                </p>
              </CardContent>
            </Card>
          </TabsContent>

          <TabsContent value="thinking" class="mt-4">
            <Card>
              <CardHeader>
                <CardTitle class="text-sm">Sequential thinking</CardTitle>
              </CardHeader>
              <CardContent class="space-y-4">
                <Card class="bg-muted/30">
                  <CardContent class="flex items-center justify-between gap-3 p-3">
                    <div class="flex flex-col gap-0.5">
                      <span class="text-sm text-foreground">Sequential thinking</span>
                      <span class="text-xs text-muted-foreground">
                        Enable chain-of-thought
                      </span>
                    </div>
                    <Switch v-model="requiresThinking" aria-label="Enable sequential thinking" />
                  </CardContent>
                </Card>

                <div class="flex flex-col gap-2">
                  <Label for="reasoning-effort" class="text-xs uppercase tracking-wider text-muted-foreground">
                    Reasoning effort
                  </Label>
                  <Select v-model="selectedEffort">
                    <SelectTrigger id="reasoning-effort" class="w-full">
                      <SelectValue placeholder="Select effort" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem v-for="level in reasoningLevels" :key="level" :value="level">
                        {{ formatReasoningLevel(level) }}
                      </SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </CardContent>
            </Card>
          </TabsContent>

          <TabsContent value="meta" class="mt-4">
            <Card>
              <CardHeader>
                <CardTitle class="text-sm">Meta</CardTitle>
              </CardHeader>
              <CardContent>
                <div class="flex flex-col gap-2">
                  <Label class="text-xs uppercase tracking-wider text-muted-foreground">
                    Step ID
                  </Label>
                  <code class="font-mono text-xs text-muted-foreground break-all">
                    {{ selectedStep.id }}
                  </code>
                </div>
              </CardContent>
            </Card>
          </TabsContent>
        </Tabs>
      </div>
    </template>

    <template v-else>
      <div class="p-4 space-y-4">
        <div class="flex items-center gap-2">
          <span class="w-1.5 h-1.5 rounded-full bg-primary" />
          <span
            class="text-xs font-semibold tracking-wider uppercase text-muted-foreground"
          >
            Workflow
          </span>
        </div>

        <WorkflowMetadataForm />

        <Card role="status">
          <CardContent class="flex flex-col gap-2 py-6 text-center">
            <span class="text-sm font-medium text-foreground">Select a step</span>
            <span class="text-xs text-muted-foreground">
              Pick a step from the timeline to edit its prompt, tools, and reasoning settings.
            </span>
          </CardContent>
        </Card>
      </div>
    </template>
    </ScrollArea>
  </aside>
</template>
