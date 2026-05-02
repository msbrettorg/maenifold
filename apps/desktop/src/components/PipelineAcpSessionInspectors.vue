<script setup lang="ts">
import { computed, onBeforeUnmount, reactive, watch } from 'vue';
import { storeToRefs } from 'pinia';
import { AlertTriangle } from 'lucide-vue-next';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import SessionInspectorPanel from './sessions/SessionInspectorPanel.vue';
import { usePipelineStore } from '../stores/pipeline';
import { useSessionStore } from '../stores/session';
import type { PromptViewNode } from '../lib/pipeline-types';

interface PromptRuntime {
  nodeId: string;
  label: string;
  agent: string | null;
  model: string | null;
  sessionId: string | null;
}

const pipelineStore = usePipelineStore();
const sessionStore = useSessionStore();
const { currentView } = storeToRefs(pipelineStore);
const localSessionIds = reactive<Record<string, string>>({});
const inspectorErrors = reactive<Record<string, string>>({});
const missingSessionSince = reactive<Record<string, number>>({});
const attachAttempts = reactive<Record<string, number>>({});
const retryTimers = new Map<string, ReturnType<typeof globalThis.setTimeout>>();
const missingSessionGraceMs = 1500;
const attachRetryMs = 1000;

const inspectablePromptNodes = computed(() =>
  (currentView.value?.nodes ?? [])
    .filter((node): node is PromptViewNode =>
      node.type === 'prompt'
      && (node.status === 'active' || Boolean(node.acpSessionId || node.provenance?.detail.sessionId))
    )
);

const promptRuntimes = computed<PromptRuntime[]>(() =>
  inspectablePromptNodes.value.map(toPromptRuntime)
);

const hasRunningPrompt = computed(() => promptRuntimes.value.length > 0);

watch(
  promptRuntimes,
  (runtimes) => {
    const activeNodeIds = new Set(runtimes.map((runtime) => runtime.nodeId));
    for (const nodeId of Object.keys(localSessionIds)) {
      if (!activeNodeIds.has(nodeId)) delete localSessionIds[nodeId];
    }
    for (const nodeId of Object.keys(inspectorErrors)) {
      if (!activeNodeIds.has(nodeId)) delete inspectorErrors[nodeId];
    }
    for (const nodeId of Object.keys(missingSessionSince)) {
      if (!activeNodeIds.has(nodeId)) delete missingSessionSince[nodeId];
    }
    for (const nodeId of Object.keys(attachAttempts)) {
      if (!activeNodeIds.has(nodeId)) delete attachAttempts[nodeId];
    }
    for (const [nodeId, timer] of retryTimers.entries()) {
      if (activeNodeIds.has(nodeId)) continue;
      globalThis.clearTimeout(timer);
      retryTimers.delete(nodeId);
    }
    for (const runtime of runtimes) {
      void ensureInspectable(runtime);
    }
  },
  { immediate: true },
);

async function ensureInspectable(runtime: PromptRuntime): Promise<void> {
  if (!runtime.sessionId) {
    const firstSeen = missingSessionSince[runtime.nodeId] ?? Date.now();
    missingSessionSince[runtime.nodeId] = firstSeen;
    if (Date.now() - firstSeen < missingSessionGraceMs) {
      scheduleRetry(runtime, missingSessionGraceMs);
      return;
    }
    inspectorErrors[runtime.nodeId] =
      `Hidden inference path: prompt node ${runtime.label} is active without an ACP session identifier.`;
    return;
  }
  delete missingSessionSince[runtime.nodeId];
  if (!runtime.agent) {
    inspectorErrors[runtime.nodeId] =
      `Hidden inference path: prompt node ${runtime.label} is active without an agent identity.`;
    return;
  }
  if (!runtime.model) {
    inspectorErrors[runtime.nodeId] =
      `Hidden inference path: prompt node ${runtime.label} is active without a model identity.`;
    return;
  }
  if (localSessionIds[runtime.nodeId]) return;
  const result = await sessionStore.focusOrResumeRemoteSession(
    runtime.sessionId,
    runtime.agent,
    { suppressErrors: true },
  );
  if (result.localSessionId && (result.status === 'activated' || result.status === 'resumed')) {
    localSessionIds[runtime.nodeId] = result.localSessionId;
    delete inspectorErrors[runtime.nodeId];
    return;
  }
  attachAttempts[runtime.nodeId] = (attachAttempts[runtime.nodeId] ?? 0) + 1;
  if (runtime.sessionId === sessionStore.connectionError?.match(/ACP session ([^ ]+)/)?.[1]) {
    scheduleRetry(runtime, attachRetryMs);
    return;
  }
  inspectorErrors[runtime.nodeId] =
    `Hidden inference path: unable to attach inspector to ACP session ${runtime.sessionId}.`;
  scheduleRetry(runtime, attachRetryMs);
}

function localSessionIdFor(runtime: PromptRuntime): string | null {
  return localSessionIds[runtime.nodeId] ?? null;
}

function setInspectorError(nodeId: string, message: string): void {
  inspectorErrors[nodeId] = message;
}

function scheduleRetry(runtime: PromptRuntime, delayMs: number): void {
  if (retryTimers.has(runtime.nodeId)) return;
  const timer = globalThis.setTimeout(() => {
    retryTimers.delete(runtime.nodeId);
    const latestRuntime = runtimeForNode(runtime.nodeId);
    if (!latestRuntime) return;
    void ensureInspectable(latestRuntime);
  }, delayMs);
  retryTimers.set(runtime.nodeId, timer);
}

function runtimeForNode(nodeId: string): PromptRuntime | null {
  const node = (currentView.value?.nodes ?? []).find((candidate): candidate is PromptViewNode =>
    candidate.type === 'prompt' && candidate.nodeId === nodeId
  );
  if (!node) return null;
  return toPromptRuntime(node);
}

function toPromptRuntime(node: PromptViewNode): PromptRuntime {
  const sessionId =
    node.provenance?.detail.sessionId?.trim()
    || node.acpSessionId?.trim()
    || node.sessionRef?.sessionId.trim()
    || null;
  const agent = node.provenance?.agent?.trim() || node.agent?.trim() || node.sessionRef?.agent.trim() || null;
  const model = node.model?.trim() || node.agentConfig?.model?.trim() || null;
  return {
    nodeId: node.nodeId,
    label: node.displayLabel ?? node.nodeId,
    agent,
    model,
    sessionId,
  };
}

onBeforeUnmount(() => {
  for (const timer of retryTimers.values()) {
    globalThis.clearTimeout(timer);
  }
  retryTimers.clear();
});
</script>

<template>
  <section v-if="hasRunningPrompt" class="flex shrink-0 flex-col gap-3 p-4" aria-label="Pipeline ACP session inspectors">
    <Card
      v-for="runtime in promptRuntimes"
      :key="runtime.nodeId"
      class="flex min-h-0 flex-col bg-surface"
    >
      <CardHeader class="flex flex-row items-start justify-between gap-3 space-y-0">
        <div class="min-w-0 space-y-2">
          <CardTitle class="truncate text-sm">{{ runtime.label }}</CardTitle>
          <div class="flex flex-wrap gap-2">
            <Badge v-if="runtime.agent" variant="secondary">{{ runtime.agent }}</Badge>
            <Badge v-if="runtime.model" variant="outline">{{ runtime.model }}</Badge>
            <Badge v-if="runtime.sessionId" variant="outline">{{ runtime.sessionId }}</Badge>
          </div>
        </div>
      </CardHeader>
      <CardContent class="flex min-h-0 flex-1 flex-col">
        <div
          v-if="inspectorErrors[runtime.nodeId]"
          class="mb-3 flex items-start gap-2 rounded-2xl bg-destructive/10 px-3 py-2 text-xs text-destructive"
          role="alert"
        >
          <AlertTriangle class="mt-0.5 size-4 shrink-0" aria-hidden="true" />
          <span>{{ inspectorErrors[runtime.nodeId] }}</span>
        </div>
        <SessionInspectorPanel
          v-if="localSessionIdFor(runtime)"
          :session-id="localSessionIdFor(runtime) ?? ''"
          @error="setInspectorError(runtime.nodeId, $event)"
        />
        <p v-else class="m-0 text-sm text-muted-foreground">
          Connecting ACP session inspector.
        </p>
      </CardContent>
    </Card>
  </section>
</template>
