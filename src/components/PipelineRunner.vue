<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue';
import { storeToRefs } from 'pinia';
import type mermaidType from 'mermaid';
import { usePipelineStore } from '../stores/pipeline';
import type { LogEntry } from '../lib/pipeline-types';

type RunnerTab = 'event-log' | 'mermaid';

const store = usePipelineStore();
const { currentPipelineId, currentView, designMode, errorMessage, eventLog, isRunning, localDefinition } = storeToRefs(store);

const activeTab = ref<RunnerTab>('event-log');
const busyAction = ref<string | null>(null);
const logContainer = ref<HTMLElement | null>(null);
const runPrompt = ref('');
const renderedSvg = ref('');
const mermaidFallback = ref('');
let mermaidPromise: Promise<typeof mermaidType> | null = null;
let mermaidInitialized = false;
let renderToken = 0;

const statusLine = computed(() => {
  if (designMode.value) {
    return `Design mode — ${localDefinition.value.nodes.length} nodes, ${localDefinition.value.edges.length} edges`;
  }
  if (!currentView.value) return 'No pipeline loaded';
  const activeList = currentView.value.active.length > 0 ? currentView.value.active.join(', ') : 'none';
  return `● ${currentView.value.status} — ${currentView.value.completed.length}/${currentView.value.nodes.length} completed — Active: ${activeList}`;
});

const canCreate = computed(() => designMode.value && localDefinition.value.nodes.length > 0);
const canRun = computed(() => !designMode.value && !isRunning.value && !!currentView.value && currentView.value.ready.length > 0);
const canShowRunPrompt = computed(() => canRun.value);
const canPause = computed(() => !designMode.value && isRunning.value);
const canResume = computed(() => !designMode.value && !isRunning.value && !!currentView.value && !['completed', 'cancelled', 'stuck'].includes(currentView.value.status));
const canCancel = computed(() => !designMode.value && (isRunning.value || currentView.value?.status === 'paused'));
const canDelete = computed(() => !designMode.value && !!currentPipelineId.value);

const viewNodesById = computed(() => new Map((currentView.value?.nodes ?? []).map((node) => [node.nodeId, node])));
const formattedLog = computed(() => eventLog.value.map((entry) => formatLogEntry(entry)));

watch(() => eventLog.value.length, async (length, previousLength) => {
  if (length === previousLength) return;
  await nextTick();
  if (logContainer.value) {
    logContainer.value.scrollTop = logContainer.value.scrollHeight;
  }
});

watch([activeTab, () => currentView.value?.mermaid ?? ''], async ([tab, diagram]) => {
  if (tab !== 'mermaid') return;
  if (!diagram) {
    renderedSvg.value = '';
    mermaidFallback.value = '';
    return;
  }
  const token = ++renderToken;
  try {
    const mermaid = await getMermaid();
    ensureMermaidInit(mermaid);
    const { svg } = await mermaid.render(`pipeline-runner-${token}`, diagram);
    if (token !== renderToken) return;
    renderedSvg.value = svg;
    mermaidFallback.value = '';
  } catch {
    if (token !== renderToken) return;
    renderedSvg.value = '';
    mermaidFallback.value = diagram;
  }
}, { immediate: true });

function iconFor(status: string): string {
  switch (status) {
    case 'active': return '▶';
    case 'completed': return '✅';
    case 'abandoned': return '❌';
    case 'stuck': return '🚫';
    case 'ready': return '🟡';
    case 'pending': return '⏳';
    default: return '•';
  }
}

function formatLogEntry(entry: LogEntry) {
  const timestamp = new Date(entry.timestamp).toLocaleTimeString([], {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false,
  });
  const nodeOutput = viewNodesById.value.get(entry.nodeId)?.output;
  if (entry.to === 'completed' && typeof nodeOutput === 'string' && nodeOutput.trim().length > 0) {
    const preview = nodeOutput.trim().replace(/\s+/g, ' ');
    const snippet = preview.slice(0, 100);
    const suffix = preview.length > 100 ? '...' : '';
    return `[${timestamp}] ✅ ${entry.nodeId} completed — output: ${snippet}${suffix}`;
  }
  return `[${timestamp}] ${iconFor(entry.to)} ${entry.nodeId}: ${entry.from} → ${entry.to}`;
}

function ensureMermaidInit(mermaid: typeof mermaidType): void {
  if (mermaidInitialized) return;
  mermaid.initialize({ startOnLoad: false, theme: 'neutral' });
  mermaidInitialized = true;
}

async function getMermaid(): Promise<typeof mermaidType> {
  if (!mermaidPromise) {
    mermaidPromise = import('mermaid').then((module) => module.default);
  }
  return mermaidPromise;
}

async function runAction(name: string, action: () => Promise<unknown>) {
  busyAction.value = name;
  try {
    await action();
  } finally {
    busyAction.value = null;
  }
}

function handleDelete() {
  if (!window.confirm('Delete this pipeline? This action cannot be undone.')) return;
  void runAction('delete', () => store.deletePipeline());
}

function handleRun() {
  void runAction('run', async () => {
    const prompt = runPrompt.value.trim();
    await store.runPipeline(prompt.length > 0 ? prompt : undefined);
    runPrompt.value = '';
  });
}
</script>

<template>
  <section class="runner-panel">
    <div v-if="canShowRunPrompt" class="prompt-panel">
      <label class="prompt-label" for="pipeline-run-prompt">Run prompt</label>
      <textarea
        id="pipeline-run-prompt"
        v-model="runPrompt"
        class="field-textarea"
        rows="3"
        placeholder="Optional: ambient context for all nodes..."
      />
    </div>

    <div v-if="errorMessage" class="error-banner" role="alert">
      <span class="error-banner__message">{{ errorMessage }}</span>
      <button class="btn btn-ghost error-banner__dismiss" type="button" aria-label="Dismiss error" @click="store.clearError()">✕</button>
    </div>

    <div class="control-bar">
      <button v-if="canCreate" class="btn btn-primary" :disabled="busyAction !== null" @click="runAction('create', () => store.createPipeline())">Create</button>
      <button v-if="canRun" class="btn btn-success" :disabled="busyAction !== null" @click="handleRun">Run</button>
      <button v-if="canPause" class="btn btn-pause" :disabled="busyAction !== null" @click="runAction('pause', () => store.pausePipeline())">Pause</button>
      <button v-if="canResume" class="btn btn-primary" :disabled="busyAction !== null" @click="runAction('resume', () => store.resumePipeline())">Resume</button>
      <button v-if="canCancel" class="btn btn-danger" :disabled="busyAction !== null" @click="runAction('cancel', () => store.cancelPipeline())">Cancel</button>
      <button v-if="canDelete" class="btn btn-danger" :disabled="busyAction !== null" @click="handleDelete">Delete</button>
    </div>

    <div class="status-line">{{ statusLine }}</div>

    <div class="tab-bar">
      <button class="btn btn-ghost tab" :class="{ active: activeTab === 'event-log' }" @click="activeTab = 'event-log'">Event Log</button>
      <button class="btn btn-ghost tab" :class="{ active: activeTab === 'mermaid' }" @click="activeTab = 'mermaid'">Mermaid</button>
    </div>

    <div v-if="activeTab === 'event-log'" ref="logContainer" class="tab-panel log-panel" aria-live="polite" aria-label="Event log">
      <p v-if="formattedLog.length === 0" class="empty-state">No events yet.</p>
      <pre v-else class="log-lines"><code>{{ formattedLog.join('\n') }}</code></pre>
    </div>

    <div v-else class="tab-panel mermaid-panel">
      <div v-if="renderedSvg" class="diagram" v-html="renderedSvg" />
      <pre v-else-if="mermaidFallback" class="fallback"><code>{{ mermaidFallback }}</code></pre>
      <p v-else class="empty-state">No Mermaid diagram available.</p>
    </div>
  </section>
</template>

<style scoped>
.runner-panel {
  display: flex;
  flex-direction: column;
  min-height: 260px;
  border-top: 1px solid var(--border-color);
  background: var(--bg-main);
}
.error-banner { display: flex; align-items: center; gap: 0.75rem; padding: 0.6rem 1rem; background: var(--bg-error); color: var(--text-error); border-bottom: 1px solid var(--border-error); font-size: var(--text-sm); }
.error-banner__message { flex: 1; }
.error-banner__dismiss { flex-shrink: 0; color: inherit; }
.error-banner__dismiss:hover { opacity: 0.75; }
.prompt-panel {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  padding: 0.75rem 1rem 0;
}
.prompt-label {
  color: var(--text-secondary);
  font-size: var(--text-sm);
  font-weight: 600;
}
.control-bar, .tab-bar {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  padding: 0.75rem 1rem;
}
.control-bar { border-bottom: 1px solid var(--border-color); }
.status-line {
  padding: 0 1rem 0.75rem;
  color: var(--text-secondary);
  font-size: var(--text-base);
}
.tab-bar {
  border-top: 1px solid var(--border-color);
  border-bottom: 1px solid var(--border-color);
  padding-top: 0;
  padding-bottom: 0;
}
.tab {
  color: var(--text-secondary);
  border-bottom: 2px solid transparent;
}
.tab.active {
  color: var(--text-accent);
  border-bottom-color: var(--text-accent);
}
.tab-panel {
  flex: 1;
  min-height: 0;
  padding: 1rem;
  overflow: auto;
}
.log-panel {
  background: var(--bg-assistant);
}
.log-lines, .fallback {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  font-family: var(--font-mono);
  font-size: var(--text-sm);
  color: var(--text-primary);
}
.diagram :deep(svg) {
  width: 100%;
  height: auto;
}
.empty-state {
  color: var(--text-muted);
}
</style>
