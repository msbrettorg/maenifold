<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import { storeToRefs } from 'pinia';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import type { WorkflowStep } from '../lib/workflow-types';
import { useWorkflowStore } from '../stores/workflow';
import WorkflowRunButton from './WorkflowRunButton.vue';
import WorkflowStepDetail from './WorkflowStepDetail.vue';
import WorkflowStopButton from './WorkflowStopButton.vue';
import type { WorkflowAssetDefinition } from '../lib/workflow-asset-types';

const props = defineProps<{
  workflowName: string;
  workflow?: WorkflowAssetDefinition | null;
}>();

interface HistoryEntry {
  heading: string;
  body: string;
}

interface LiveStep {
  step: WorkflowStep;
  label: 'pending' | 'running' | 'completed' | 'failed';
}

const workflowStore = useWorkflowStore();
const { selectedSession } = storeToRefs(workflowStore);
const expandedSteps = ref<Set<number>>(new Set());

function parseStepNumber(heading: string): number | null {
  const match = /^Step\s+(\d+)\//.exec(heading.trim());
  return match?.[1] ? Number(match[1]) : null;
}

function isResponse(heading: string): boolean {
  return /\bResponse\b/.test(heading);
}

function titleFromBody(body: string, fallback: string): string {
  return body.split('\n').find((line) => line.trim().length > 0)?.trim() ?? fallback;
}

function authoredStep(stepNumber: number) {
  return props.workflow?.steps?.[stepNumber - 1] ?? null;
}

function nameForStep(stepNumber: number, prompt: HistoryEntry | undefined): string {
  const authored = authoredStep(stepNumber);
  if (authored) return authored.name;
  return titleFromBody(prompt?.body ?? '', `Step ${stepNumber}`);
}

function labelForStep(stepNumber: number): LiveStep['label'] {
  const session = selectedSession.value;
  if (!session) return 'pending';
  if (session.status === 'completed') return 'completed';
  if (session.status === 'cancelled' || session.status === 'abandoned') return 'failed';
  const current = session.current_step ?? 1;
  if (stepNumber < current) return 'completed';
  if (stepNumber === current) return 'running';
  return 'pending';
}

function statusForLabel(label: LiveStep['label']): WorkflowStep['status'] {
  if (label === 'completed') return 'completed';
  if (label === 'running') return 'active';
  return 'pending';
}

const liveSteps = computed<LiveStep[]>(() => {
  const session = selectedSession.value;
  if (!session) return [];
  const prompts = new Map<number, HistoryEntry>();
  const responses = new Map<number, HistoryEntry>();
  for (const entry of session.history ?? []) {
    const number = parseStepNumber(entry.heading);
    if (!number) continue;
    if (isResponse(entry.heading)) responses.set(number, entry);
    else prompts.set(number, entry);
  }

  const total = Math.max(session.total_steps ?? 0, props.workflow?.steps?.length ?? 0, prompts.size, responses.size);
  return Array.from({ length: total }, (_, index) => {
    const stepNumber = index + 1;
    const prompt = prompts.get(stepNumber);
    const response = responses.get(stepNumber);
    const label = labelForStep(stepNumber);
    const authored = authoredStep(stepNumber);
    return {
      label,
      step: {
        stepNumber,
        totalSteps: total,
        id: authored?.id ?? `step-${stepNumber}`,
        name: nameForStep(stepNumber, prompt),
        status: statusForLabel(label),
        response: response?.body,
        respondedAt: response ? (session.updated_at ?? undefined) : undefined,
        requiresSequentialThinking: authored?.requiresSequentialThinking ?? false,
      },
    };
  });
});

const isComplete = computed(() => {
  const status = selectedSession.value?.status;
  return status === 'completed' || status === 'cancelled' || status === 'abandoned';
});

const outcome = computed(() => {
  if (selectedSession.value?.status === 'completed') return 'Run completed';
  if (selectedSession.value?.status === 'cancelled') return 'Run cancelled';
  if (selectedSession.value?.status === 'abandoned') return 'Run failed';
  return null;
});

const totalDuration = computed(() => {
  const session = selectedSession.value;
  if (!session?.started_at || !session.updated_at) return '—';
  const ms = new Date(session.updated_at).getTime() - new Date(session.started_at).getTime();
  if (!Number.isFinite(ms) || ms < 0) return '—';
  const seconds = Math.max(1, Math.round(ms / 1000));
  return seconds < 60 ? `${seconds}s` : `${Math.floor(seconds / 60)}m ${seconds % 60}s`;
});

function isExpanded(stepNumber: number): boolean {
  return expandedSteps.value.has(stepNumber);
}

function toggle(stepNumber: number) {
  const next = new Set(expandedSteps.value);
  if (next.has(stepNumber)) next.delete(stepNumber);
  else next.add(stepNumber);
  expandedSteps.value = next;
}

function badgeVariant(label: LiveStep['label']): 'default' | 'secondary' | 'destructive' | 'outline' {
  if (label === 'completed') return 'default';
  if (label === 'running') return 'secondary';
  if (label === 'failed') return 'destructive';
  return 'outline';
}

onMounted(() => {
  void workflowStore.mount();
});

onBeforeUnmount(() => {
  workflowStore.unmount();
});
</script>

<template>
  <section class="flex h-full min-w-0 flex-1 flex-col bg-bg-app" aria-label="Workflow live progress">
    <header class="flex items-start justify-between gap-4 border-b border-border p-4">
      <div class="min-w-0">
        <h1 class="m-0 truncate text-lg font-semibold text-text-primary">{{ workflowName }}</h1>
        <p class="m-0 text-xs text-text-secondary">
          Session {{ selectedSession?.id ?? 'starting…' }}
        </p>
      </div>
      <WorkflowStopButton
        v-if="selectedSession && !isComplete"
        :session-id="selectedSession.id"
      />
      <WorkflowRunButton
        v-else-if="workflow"
        :workflow="workflow"
      />
    </header>

    <ScrollArea class="min-h-0 flex-1">
      <div class="flex flex-col gap-3 p-4">
        <Card v-if="outcome" role="status">
          <CardHeader>
            <CardTitle class="text-sm">{{ outcome }}</CardTitle>
          </CardHeader>
          <CardContent class="text-sm text-muted-foreground">
            Total duration: {{ totalDuration }}
          </CardContent>
        </Card>

        <ol class="m-0 flex list-none flex-col gap-2 p-0">
          <li v-for="{ step, label } in liveSteps" :key="step.stepNumber" class="flex flex-col gap-1">
            <div class="flex items-center justify-between gap-2">
              <Badge :variant="badgeVariant(label)" class="capitalize">{{ label }}</Badge>
              <span class="text-xs text-muted-foreground">
                Updated: {{ step.respondedAt ? totalDuration : '—' }}
              </span>
            </div>
            <WorkflowStepDetail
              :step="step"
              :expanded="isExpanded(step.stepNumber)"
              @toggle="toggle(step.stepNumber)"
            />
          </li>
        </ol>
      </div>
    </ScrollArea>
  </section>
</template>
