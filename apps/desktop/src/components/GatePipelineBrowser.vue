<script setup lang="ts">
import { computed, nextTick, ref } from 'vue';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import { formatStageLabel } from '../lib/gate-display';

type GateOutcome = 'pass' | 'fail';

type GatePipelineState = {
  terminal?: boolean;
  on?: Partial<Record<GateOutcome, string>>;
};

type GatePipelineDefinition = {
  schemaVersion: 1;
  id: string;
  name: string;
  initial: string;
  states: Record<string, GatePipelineState>;
};

const pipelineDefinitions: GatePipelineDefinition[] = [
  {
    schemaVersion: 1,
    id: 'sdd',
    name: 'Spec Driven Development',
    initial: 'pending',
    states: {
      pending: { on: { pass: 'swe' } },
      swe: { on: { fail: 'pending', pass: 'red-team' } },
      'red-team': { on: { fail: 'pending', pass: 'blue-team' } },
      'blue-team': { on: { fail: 'pending', pass: 'done' } },
      done: { terminal: true },
    },
  },
  {
    schemaVersion: 1,
    id: 'tdd',
    name: 'Test Driven Development',
    initial: 'pending',
    states: {
      pending: { on: { pass: 'blue-team' } },
      'blue-team': { on: { fail: 'pending', pass: 'swe' } },
      swe: { on: { fail: 'pending', pass: 'red-team' } },
      'red-team': { on: { fail: 'pending', pass: 'done' } },
      done: { terminal: true },
    },
  },
  {
    schemaVersion: 1,
    id: 'gsd',
    name: 'Get Stuff Done',
    initial: 'pending',
    states: {
      pending: { on: { pass: 'research' } },
      research: { on: { fail: 'pending', pass: 'plan' } },
      plan: { on: { fail: 'pending', pass: 'execute' } },
      execute: { on: { fail: 'pending', pass: 'verify' } },
      verify: { on: { fail: 'execute', pass: 'done' } },
      done: { terminal: true },
    },
  },
  {
    schemaVersion: 1,
    id: 'lightweight',
    name: 'Lightweight',
    initial: 'pending',
    states: {
      pending: { on: { pass: 'swe' } },
      swe: { on: { fail: 'pending', pass: 'blue-team' } },
      'blue-team': { on: { fail: 'pending', pass: 'done' } },
      done: { terminal: true },
    },
  },
];

const selectedPipelineId = ref(pipelineDefinitions[0]?.id ?? '');
const pipelineButtonRefs = ref<Array<InstanceType<typeof Button> | null>>([]);

const selectedPipeline = computed(() => (
  pipelineDefinitions.find((pipeline) => pipeline.id === selectedPipelineId.value)
  ?? pipelineDefinitions[0]
));

const selectedStateEntries = computed(() => Object.entries(selectedPipeline.value?.states ?? {}));

function transitionEntries(state: GatePipelineState): Array<[GateOutcome, string]> {
  return (Object.entries(state.on ?? {}) as Array<[GateOutcome, string]>)
    .sort(([left], [right]) => left.localeCompare(right));
}

function selectPipeline(id: string): void {
  selectedPipelineId.value = id;
}

async function focusPipeline(offset: number): Promise<void> {
  const currentIndex = pipelineDefinitions.findIndex((pipeline) => pipeline.id === selectedPipelineId.value);
  const safeIndex = currentIndex >= 0 ? currentIndex : 0;
  const nextIndex = (safeIndex + offset + pipelineDefinitions.length) % pipelineDefinitions.length;
  selectedPipelineId.value = pipelineDefinitions[nextIndex].id;
  await nextTick();
  const button = pipelineButtonRefs.value[nextIndex]?.$el;
  if (button instanceof HTMLElement) button.focus();
}
</script>

<template>
  <Card size="sm" class="shrink-0 rounded-lg border border-border-subtle bg-bg-surface shadow-none">
    <CardHeader class="px-4">
      <div class="flex flex-wrap items-start justify-between gap-3">
        <div class="grid gap-1">
          <CardTitle id="gate-pipeline-browser-title" class="text-body-sm font-semibold text-text-primary">
            Gate process pipelines
          </CardTitle>
          <p class="text-tiny text-text-secondary">
            Built-in process definitions. Select a pipeline to inspect its read-only state diagram.
          </p>
        </div>
        <Badge variant="outline" class="font-mono text-tiny">
          {{ pipelineDefinitions.length }} installed
        </Badge>
      </div>
    </CardHeader>
    <CardContent class="grid gap-3 px-4 lg:grid-cols-[15rem_minmax(0,1fr)]">
      <div
        role="listbox"
        aria-labelledby="gate-pipeline-browser-title"
        aria-orientation="vertical"
        class="flex flex-col gap-2"
        @keydown.down.prevent="focusPipeline(1)"
        @keydown.up.prevent="focusPipeline(-1)"
        @keydown.home.prevent="focusPipeline(-pipelineDefinitions.length)"
        @keydown.end.prevent="focusPipeline(pipelineDefinitions.length - 1)"
      >
        <Button
          v-for="(pipeline, index) in pipelineDefinitions"
          :key="pipeline.id"
          :ref="(el) => { pipelineButtonRefs[index] = el as InstanceType<typeof Button> | null; }"
          type="button"
          variant="ghost"
          role="option"
          :data-pipeline-id="pipeline.id"
          :aria-selected="selectedPipelineId === pipeline.id"
          class="h-auto justify-start rounded-md px-3 py-2 text-left"
          :class="selectedPipelineId === pipeline.id ? 'bg-brand/10 text-text-primary ring-1 ring-brand/40' : 'text-text-secondary hover:bg-bg-surface-hover hover:text-text-primary'"
          @click="selectPipeline(pipeline.id)"
        >
          <span class="grid min-w-0 gap-0.5">
            <span class="truncate text-sm font-medium">{{ pipeline.name }}</span>
            <span class="font-mono text-tiny text-text-tertiary">{{ pipeline.id }}</span>
          </span>
        </Button>
      </div>

      <section
        v-if="selectedPipeline"
        class="min-w-0 rounded-lg border border-border-subtle bg-bg-app p-3"
        :aria-label="`${selectedPipeline.name} state diagram`"
      >
        <div class="mb-3 flex flex-wrap items-center justify-between gap-2">
          <div>
            <h3 class="text-sm font-semibold text-text-primary">{{ selectedPipeline.name }}</h3>
            <p class="font-mono text-tiny text-text-secondary">
              {{ selectedPipeline.id }} · schema {{ selectedPipeline.schemaVersion }} · initial {{ selectedPipeline.initial }}
            </p>
          </div>
        </div>
        <ScrollArea class="max-h-64 pr-2">
          <div role="list" class="grid gap-2">
            <article
              v-for="[stateName, state] in selectedStateEntries"
              :key="stateName"
              role="listitem"
              class="rounded-md border border-border-subtle bg-bg-surface px-3 py-2"
            >
              <div class="flex flex-wrap items-center justify-between gap-2">
                <h4 class="font-mono text-sm font-semibold text-text-primary">
                  {{ formatStageLabel(stateName) }}
                </h4>
                <Badge v-if="stateName === selectedPipeline.initial" variant="secondary" class="text-tiny">
                  initial
                </Badge>
                <Badge v-else-if="state.terminal" variant="outline" class="text-tiny">
                  terminal
                </Badge>
              </div>
              <div v-if="transitionEntries(state).length" class="mt-2 flex flex-wrap gap-2">
                <span
                  v-for="[outcome, target] in transitionEntries(state)"
                  :key="`${stateName}-${outcome}`"
                  class="rounded-md bg-bg-elevated px-2 py-1 font-mono text-tiny text-text-secondary"
                >
                  {{ outcome }} → {{ target }}
                </span>
              </div>
              <p v-else class="mt-2 text-tiny text-text-secondary">No outgoing transitions.</p>
            </article>
          </div>
        </ScrollArea>
      </section>
    </CardContent>
  </Card>
</template>
