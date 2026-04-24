<script setup lang="ts">
import { computed } from 'vue';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import {
  Loader2,
  Rocket,
  Download,
  Package,
  Hammer,
  Settings,
  Link2,
  CheckCircle2,
  XCircle,
  Hourglass,
  ChevronDown,
  ChevronUp,
} from 'lucide-vue-next';

const props = defineProps<{
  agentName: string;
  phase: string;
  logs: string[];
  elapsedSeconds: number;
  showDetails: boolean;
}>();

const emit = defineEmits<{
  cancel: [];
  toggleDetails: [];
}>();

const phaseIconComponent = computed(() => {
  switch (props.phase) {
    case 'starting': return Rocket;
    case 'downloading': return Download;
    case 'installing': return Package;
    case 'building': return Hammer;
    case 'initializing': return Settings;
    case 'connecting': return Link2;
    case 'ready': return CheckCircle2;
    case 'agent_closed':
    case 'error': return XCircle;
    default: return Hourglass;
  }
});

type StartupStatus = 'spawning' | 'initializing' | 'ready' | 'error';

const status = computed<StartupStatus>(() => {
  if (props.phase === 'agent_closed' || props.phase === 'error') return 'error';
  if (props.phase === 'ready') return 'ready';
  if (props.phase === 'initializing') return 'initializing';
  return 'spawning';
});

const phaseText = computed(() => {
  switch (props.phase) {
    case 'starting': return 'Starting process...';
    case 'downloading': return 'Downloading packages...';
    case 'installing': return 'Installing dependencies...';
    case 'building': return 'Building...';
    case 'initializing': return 'Initializing agent...';
    case 'connecting': return 'Connecting to agent...';
    case 'ready': return 'Agent ready.';
    case 'agent_closed':
    case 'error': return 'Startup failed. Agent closed unexpectedly.';
    default: return 'Please wait...';
  }
});

const formattedTime = computed(() => {
  const mins = Math.floor(props.elapsedSeconds / 60);
  const secs = props.elapsedSeconds % 60;
  if (mins > 0) {
    return `${mins}m ${secs}s`;
  }
  return `${secs}s`;
});

const isLongWait = computed(() => props.elapsedSeconds > 10);

const progressValue = computed(() => {
  switch (props.phase) {
    case 'starting': return 10;
    case 'downloading': return 25;
    case 'installing': return 50;
    case 'building': return 70;
    case 'initializing': return 85;
    case 'connecting': return 95;
    case 'ready': return 100;
    default: return 0;
  }
});

const isInProgress = computed(
  () => status.value === 'spawning' || status.value === 'initializing',
);

const badgeClass = computed(() => {
  switch (status.value) {
    case 'ready':
      return 'bg-success text-text-on-accent border-transparent';
    case 'error':
      return 'bg-danger text-text-on-accent border-transparent';
    case 'initializing':
      return 'bg-warning text-text-on-accent border-transparent';
    default:
      return 'bg-accent-prompt text-text-on-accent border-transparent';
  }
});

const progressBarColorClass = computed(() => {
  switch (status.value) {
    case 'ready': return 'bg-success';
    case 'error': return 'bg-danger';
    case 'initializing': return 'bg-warning';
    default: return 'bg-accent-prompt';
  }
});
</script>

<template>
  <Card size="sm" class="mt-3">
    <CardContent class="flex flex-col gap-3">
      <div class="flex items-center justify-between gap-2">
        <div class="flex items-center gap-2 min-w-0">
          <Loader2
            v-if="isInProgress"
            class="size-4 shrink-0 text-text-secondary animate-spin"
            aria-hidden="true"
          />
          <span class="font-semibold text-text-primary truncate">
            Connecting to {{ agentName }}...
          </span>
        </div>
        <div class="inline-flex items-center gap-2 shrink-0">
          <Badge :class="badgeClass">{{ status }}</Badge>
          <span class="text-xs text-text-secondary font-mono">{{ formattedTime }}</span>
        </div>
      </div>

      <div
        class="flex items-center gap-2 p-2 bg-surface-high rounded-md"
      >
        <component
          :is="phaseIconComponent"
          :class="[
            'size-4 shrink-0 text-text-secondary',
            isInProgress && phaseIconComponent === Loader2 ? 'animate-spin' : '',
          ]"
          aria-hidden="true"
        />
        <span class="text-text-secondary text-sm flex-1 truncate">{{ phaseText }}</span>
        <Progress
          :model-value="progressValue"
          :indicator-class="progressBarColorClass"
          :aria-label="`Startup progress: ${phaseText}`"
          class="hidden sm:block w-24 h-1.5 bg-surface-container"
        />
      </div>

      <div
        v-if="isLongWait && !showDetails"
        class="text-xs text-text-secondary italic p-2 rounded-md bg-warning/20"
      >
        First run may take longer while dependencies are installed.
      </div>

      <div class="flex gap-2 justify-between">
        <Button
          variant="ghost"
          size="sm"
          class="text-xs text-text-secondary"
          @click="emit('toggleDetails')"
        >
          <ChevronUp v-if="showDetails" class="size-3" aria-hidden="true" />
          <ChevronDown v-else class="size-3" aria-hidden="true" />
          {{ showDetails ? 'Hide details' : 'Show details' }}
        </Button>
        <Button
          variant="ghost"
          size="sm"
          class="text-xs text-text-secondary hover:bg-destructive/20 hover:text-destructive"
          @click="emit('cancel')"
        >
          Cancel
        </Button>
      </div>

      <div v-if="showDetails" class="rounded-md overflow-hidden border border-border-subtle">
        <div class="py-1.5 px-2 bg-surface-high text-xs font-semibold text-text-secondary">
          Output
        </div>
        <div class="max-h-[150px] overflow-y-auto p-2 bg-surface-high font-mono text-xs">
          <div
            v-for="(log, index) in logs.slice(-50)"
            :key="index"
            class="text-text-secondary whitespace-pre-wrap break-all leading-snug"
          >
            {{ log }}
          </div>
          <div v-if="logs.length === 0" class="text-text-secondary italic">
            Waiting for output...
          </div>
        </div>
      </div>
    </CardContent>
  </Card>
</template>
