<script setup lang="ts">
import { Brain, FlaskConical, Search, Ruler, Settings, RefreshCw, AlertTriangle, CheckCircle2 } from 'lucide-vue-next';
import { computed, ref } from 'vue';
import type { Component } from 'vue';
import type { WorkflowSession } from '../lib/workflow-types';
import WorkflowStepDetail from './WorkflowStepDetail.vue';
import { Card, CardContent, CardFooter, CardHeader } from './ui/card';
import { Badge } from './ui/badge';
import { Progress } from './ui/progress';

const props = defineProps<{
  node: WorkflowSession;
}>();

// ─── Collapsed step tracking ─────────────────────────────────────────────────
const expandedSteps = ref<Set<number>>(new Set());

function toggleStep(stepNumber: number): void {
  if (expandedSteps.value.has(stepNumber)) {
    expandedSteps.value.delete(stepNumber);
  } else {
    expandedSteps.value.add(stepNumber);
  }
}

function isStepExpanded(stepNumber: number): boolean {
  return expandedSteps.value.has(stepNumber);
}

// ─── Status helpers ──────────────────────────────────────────────────────────
function statusIcon(status: string): Component {
  switch (status) {
    case 'abandoned': return AlertTriangle;
    case 'completed': return CheckCircle2;
    case 'active': return RefreshCw;
    default: return RefreshCw;
  }
}

const borderClass = computed(() => {
  switch (props.node.status) {
    case 'completed': return 'border-l-emerald-500';
    case 'abandoned': return 'border-l-amber-500';
    case 'active':    return 'border-l-sky-500';
    default:          return 'border-l-sky-500';
  }
});

const statusBadgeVariant = computed<'default' | 'secondary' | 'destructive' | 'outline'>(() => {
  switch (props.node.status) {
    case 'completed': return 'default';
    case 'abandoned': return 'destructive';
    case 'active':    return 'secondary';
    default:          return 'outline';
  }
});

const progressBarClass = computed(() => {
  switch (props.node.status) {
    case 'completed': return 'bg-emerald-500';
    case 'abandoned': return 'bg-amber-500';
    case 'active':    return 'bg-sky-500';
    default:          return 'bg-sky-500';
  }
});

// ─── Session ID truncation ────────────────────────────────────────────────────
const shortSessionId = computed(() => {
  const id = props.node.sessionId;
  if (id.length <= 16) return id;
  return id.slice(0, 8) + '…' + id.slice(-6);
});

// ─── Progress bar ─────────────────────────────────────────────────────────────
const completedSteps = computed(() =>
  props.node.steps.filter((s) => s.status === 'completed').length
);

const totalSteps = computed(() => props.node.steps.length);

const progressPercent = computed(() =>
  totalSteps.value === 0 ? 0 : Math.round((completedSteps.value / totalSteps.value) * 100)
);

// ─── Workflow name / emoji ────────────────────────────────────────────────────
function nodeTypeIcon(name: string): Component {
  if (name.includes('think')) return Brain;
  if (name.includes('research')) return FlaskConical;
  if (name.includes('review')) return Search;
  if (name.includes('plan')) return Ruler;
  return Settings;
}

const workflowIcon = computed(() => {
  const name = props.node.workflowName ?? props.node.workflowId;
  return nodeTypeIcon(name);
});

// ─── Timing ──────────────────────────────────────────────────────────────────
const startedFormatted = computed(() => formatAbsolute(props.node.started));

const durationFormatted = computed((): string | null => {
  if (props.node.status !== 'completed') return null;
  const lastStep = [...props.node.steps]
    .filter((s) => s.respondedAt)
    .sort((a, b) => new Date(b.respondedAt!).getTime() - new Date(a.respondedAt!).getTime())[0];
  if (!lastStep?.respondedAt) return null;
  const ms = new Date(lastStep.respondedAt).getTime() - new Date(props.node.started).getTime();
  return formatDuration(ms);
});

function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  const s = Math.round(ms / 1000);
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60);
  const rem = s % 60;
  return rem > 0 ? `${m}m ${rem}s` : `${m}m`;
}

function formatAbsolute(iso: string): string {
  try {
    return new Date(iso).toLocaleString(undefined, {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  } catch {
    return iso;
  }
}
</script>

<template>
  <Card
    role="listitem"
    :class="['w-80 gap-2 py-3.5 border-l-[3px]', borderClass]"
  >
    <CardHeader class="px-4 [.border-b]:pb-0">
      <div class="flex items-center justify-between gap-2">
        <div class="flex items-center gap-2 min-w-0">
          <component
            :is="workflowIcon"
            class="size-4 shrink-0 text-muted-foreground"
            :stroke-width="1.5"
          />
          <div class="flex flex-col min-w-0">
            <span class="text-xs font-semibold truncate text-foreground">
              {{ node.workflowName || node.workflowId }}
            </span>
            <span
              class="font-mono text-[10px] truncate text-muted-foreground"
              :title="node.sessionId"
            >
              {{ shortSessionId }}
            </span>
          </div>
        </div>
        <Badge :variant="statusBadgeVariant" class="gap-1">
          <component
            :is="statusIcon(node.status)"
            class="size-3 shrink-0"
            :class="node.status === 'active' ? 'animate-spin' : ''"
            :stroke-width="2"
          />
          <span class="capitalize">{{ node.status }}</span>
        </Badge>
      </div>
    </CardHeader>

    <CardContent class="px-4 flex flex-col gap-2">
      <!-- Progress bar -->
      <Progress
        :model-value="progressPercent"
        :indicator-class="progressBarClass"
        :title="`${completedSteps}/${totalSteps} steps`"
        :aria-label="`${completedSteps} of ${totalSteps} steps complete`"
        class="h-1 w-full"
      />
      <div class="text-[10px] text-right text-muted-foreground -mt-1">
        {{ completedSteps }}/{{ totalSteps }} steps
      </div>

      <!-- Step list -->
      <ul class="list-none p-0 m-0 flex flex-col gap-0">
        <WorkflowStepDetail
          v-for="step in node.steps"
          :key="step.stepNumber"
          :step="step"
          :expanded="isStepExpanded(step.stepNumber)"
          @toggle="toggleStep(step.stepNumber)"
        />
      </ul>
    </CardContent>

    <CardFooter class="px-4 pt-1.5 border-t border-border/40 flex gap-4">
      <span class="flex flex-col gap-0.5">
        <span class="text-[10px] tracking-wider uppercase text-muted-foreground">Started</span>
        <span class="text-[10px] text-foreground">{{ startedFormatted }}</span>
      </span>
      <span v-if="durationFormatted" class="flex flex-col gap-0.5">
        <span class="text-[10px] tracking-wider uppercase text-muted-foreground">Duration</span>
        <span class="text-[10px] text-foreground">{{ durationFormatted }}</span>
      </span>
    </CardFooter>
  </Card>
</template>
