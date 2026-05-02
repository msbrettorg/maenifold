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
  status: string;
  agent: string | null;
  model: string | null;
  sessionId: string | null;
}

const pipelineStore = usePipelineStore();
const sessionStore = useSessionStore();
const { currentView } = storeToRefs(pipelineStore);
const localSessionIds = reactive<Record<string, string>>({});
const attachedRemoteSessionIds = reactive<Record<string, string>>({});
const inspectorErrors = reactive<Record<string, string>>({});
const missingSessionSince = reactive<Record<string, number>>({});
const attachAttempts = reactive<Record<string, number>>({});
const failedAttachSessionIds = reactive<Record<string, string>>({});
const attachingSessionIds = reactive<Record<string, string>>({});
const retryTimers = new Map<string, ReturnType<typeof globalThis.setTimeout>>();
const missingSessionGraceMs = 1500;

const inspectablePromptNodes = computed(() =>
  (currentView.value?.nodes ?? [])
    .filter((node): node is PromptViewNode =>
      node.type === 'prompt'
      && node.status === 'active'
    )
);

const promptRuntimes = computed<PromptRuntime[]>(() =>
  inspectablePromptNodes.value.map(toPromptRuntime)
);

const hasRunningPrompt = computed(() => promptRuntimes.value.length > 0);

watch(
  promptRuntimes,
  (runtimes) => {
    const activeRuntimes = new Map(runtimes.map((runtime) => [runtime.nodeId, runtime]));
    for (const nodeId of Object.keys(localSessionIds)) {
      const runtime = activeRuntimes.get(nodeId);
      if (!runtime || attachedRemoteSessionIds[nodeId] !== runtime.sessionId) {
        closeLocalInspector(nodeId);
      }
    }
    for (const nodeId of Object.keys(attachedRemoteSessionIds)) {
      if (!activeRuntimes.has(nodeId)) delete attachedRemoteSessionIds[nodeId];
    }
    for (const nodeId of Object.keys(inspectorErrors)) {
      if (!activeRuntimes.has(nodeId)) delete inspectorErrors[nodeId];
    }
    for (const nodeId of Object.keys(missingSessionSince)) {
      if (!activeRuntimes.has(nodeId)) delete missingSessionSince[nodeId];
    }
    for (const nodeId of Object.keys(attachAttempts)) {
      if (!activeRuntimes.has(nodeId)) delete attachAttempts[nodeId];
    }
    for (const nodeId of Object.keys(failedAttachSessionIds)) {
      if (!activeRuntimes.has(nodeId)) delete failedAttachSessionIds[nodeId];
    }
    for (const nodeId of Object.keys(attachingSessionIds)) {
      if (!activeRuntimes.has(nodeId)) delete attachingSessionIds[nodeId];
    }
    for (const [nodeId, timer] of retryTimers.entries()) {
      if (activeRuntimes.has(nodeId)) continue;
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
  if (failedAttachSessionIds[runtime.nodeId] === runtime.sessionId) return;
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
  if (attachingSessionIds[runtime.nodeId] === runtime.sessionId) return;
  attachingSessionIds[runtime.nodeId] = runtime.sessionId;
  const prefix = runtime.status === 'active'
    ? 'Hidden inference path'
    : 'ACP transcript unavailable';
  let result: Awaited<ReturnType<typeof sessionStore.focusOrResumeRemoteSession>>;
  try {
    result = await sessionStore.focusOrResumeRemoteSession(
      runtime.sessionId,
      runtime.agent,
      { suppressErrors: true },
    );
  } catch (error) {
    failedAttachSessionIds[runtime.nodeId] = runtime.sessionId;
    const message = error instanceof Error ? error.message : String(error);
    inspectorErrors[runtime.nodeId] =
      `${prefix}: unable to attach inspector to ACP session ${runtime.sessionId}: ${message}`;
    return;
  } finally {
    if (attachingSessionIds[runtime.nodeId] === runtime.sessionId) {
      delete attachingSessionIds[runtime.nodeId];
    }
  }
  if (result.localSessionId && (result.status === 'activated' || result.status === 'resumed')) {
    localSessionIds[runtime.nodeId] = result.localSessionId;
    attachedRemoteSessionIds[runtime.nodeId] = runtime.sessionId;
    delete inspectorErrors[runtime.nodeId];
    return;
  }
  attachAttempts[runtime.nodeId] = (attachAttempts[runtime.nodeId] ?? 0) + 1;
  failedAttachSessionIds[runtime.nodeId] = runtime.sessionId;
  inspectorErrors[runtime.nodeId] =
    `${prefix}: unable to attach inspector to ACP session ${runtime.sessionId}.`;
}

function localSessionIdFor(runtime: PromptRuntime): string | null {
  return localSessionIds[runtime.nodeId] ?? null;
}

function setInspectorError(nodeId: string, message: string): void {
  inspectorErrors[nodeId] = message;
}

function closeLocalInspector(nodeId: string): void {
  const localSessionId = localSessionIds[nodeId];
  delete localSessionIds[nodeId];
  delete attachedRemoteSessionIds[nodeId];
  if (!localSessionId) return;
  void sessionStore.closeSession(localSessionId).catch((error: unknown) => {
    const message = error instanceof Error ? error.message : String(error);
    inspectorErrors[nodeId] = `ACP inspector cleanup failed: ${message}`;
  });
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
    status: node.status,
    agent,
    model,
    sessionId,
  };
}

onBeforeUnmount(() => {
  for (const nodeId of Object.keys(localSessionIds)) {
    closeLocalInspector(nodeId);
  }
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
