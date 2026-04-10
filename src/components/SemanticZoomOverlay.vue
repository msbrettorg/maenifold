<script setup lang="ts">
import { computed, onMounted, onUnmounted } from 'vue';
import { useSemanticZoomStore } from '../stores/semantic-zoom';
import { usePipelineStore } from '../stores/pipeline';
import type { ViewNode } from '../lib/pipeline-types';

const zoom = useSemanticZoomStore();
const pipeline = usePipelineStore();

const viewNode = computed<ViewNode | undefined>(() => {
  if (!zoom.focusedNodeId || !pipeline.currentView) return undefined;
  return pipeline.currentView.nodes.find((n) => n.nodeId === zoom.focusedNodeId);
});

const nodeType = computed(() => viewNode.value?.type ?? 'workflow');
const statusLabel = computed(() => viewNode.value?.status ?? 'pending');

const assembledPrompt = computed(() => {
  const node = viewNode.value;
  if (!node) return null;
  if ('assembled_prompt' in node) return node.assembled_prompt ?? null;
  return null;
});

const thinkingTrace = computed(() => {
  const node = viewNode.value;
  if (!node || node.type !== 'sequentialThinking') return null;
  return node.thinking_trace ?? null;
});

const workflowSteps = computed(() => {
  const node = viewNode.value;
  if (!node || (node.type !== undefined && node.type !== 'workflow')) return null;
  if ('workflow_steps' in node) return node.workflow_steps ?? null;
  return null;
});

const nodeOutput = computed(() => viewNode.value?.output ?? null);

function handleKeyDown(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    zoom.back();
  }
}

onMounted(() => document.addEventListener('keydown', handleKeyDown));
onUnmounted(() => document.removeEventListener('keydown', handleKeyDown));
</script>

<template>
  <Teleport to="body">
    <div v-if="zoom.isInspecting && viewNode" class="zoom-overlay" @click.self="zoom.back()">
      <div class="zoom-panel">
        <header class="zoom-header">
          <div class="zoom-header-left">
            <span class="zoom-node-id">{{ viewNode.nodeId }}</span>
            <span :class="['zoom-type-pill', `zoom-type--${nodeType}`]">{{ nodeType }}</span>
            <span :class="['zoom-status', `zoom-status--${statusLabel}`]">{{ statusLabel }}</span>
          </div>
          <button class="zoom-close" @click="zoom.back()" aria-label="Close inspection" title="Close (Esc)">
            ×
          </button>
        </header>

        <div class="zoom-body">
          <details v-if="assembledPrompt" class="zoom-section">
            <summary class="zoom-section-title">Assembled Prompt</summary>
            <pre class="zoom-pre">{{ assembledPrompt }}</pre>
          </details>

          <details v-if="thinkingTrace && thinkingTrace.length > 0" class="zoom-section" open>
            <summary class="zoom-section-title">Thinking Trace ({{ thinkingTrace.length }} thoughts)</summary>
            <ol class="zoom-list">
              <li v-for="thought in thinkingTrace" :key="thought.thought_number" class="zoom-list-item">
                <span class="item-label">#{{ thought.thought_number }}</span>
                <span :class="['item-status', `zoom-status--${thought.status}`]">{{ thought.status }}</span>
                <pre class="zoom-pre">{{ thought.text }}</pre>
              </li>
            </ol>
          </details>

          <details v-if="workflowSteps && workflowSteps.length > 0" class="zoom-section" open>
            <summary class="zoom-section-title">Workflow Steps ({{ workflowSteps.length }})</summary>
            <ol class="zoom-list">
              <li v-for="step in workflowSteps" :key="step.step_name" class="zoom-list-item">
                <span class="item-label">{{ step.step_name }}</span>
                <span :class="['item-status', `zoom-status--${step.status}`]">{{ step.status }}</span>
                <pre v-if="step.output" class="zoom-pre">{{ step.output }}</pre>
              </li>
            </ol>
          </details>

          <details v-if="nodeOutput" class="zoom-section" open>
            <summary class="zoom-section-title">Output</summary>
            <pre class="zoom-pre">{{ nodeOutput }}</pre>
          </details>

          <div v-if="!assembledPrompt && !thinkingTrace && !workflowSteps && !nodeOutput" class="zoom-empty">
            This node hasn't produced any output yet.
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.zoom-overlay {
  position: fixed; inset: 0; z-index: 1000;
  display: grid; place-items: center;
  background: rgba(0, 0, 0, 0.6);
}
.zoom-panel {
  width: min(720px, 90vw); max-height: 80vh;
  display: flex; flex-direction: column;
  background: var(--surface-container, #201f1f);
  border-radius: var(--radius-xl); overflow: hidden;
}
.zoom-header {
  display: flex; align-items: center; justify-content: space-between;
  padding: var(--space-3) var(--space-4);
  background: var(--surface-container-highest, #353534);
}
.zoom-header-left { display: flex; align-items: center; gap: var(--space-2); }
.zoom-node-id { font-family: var(--font-mono); font-size: var(--text-sm); font-weight: 600; color: var(--text-primary); }
.zoom-type-pill {
  font-size: var(--text-xs); padding: 2px 8px; border-radius: 999px;
  color: var(--text-primary);
  background: color-mix(in srgb, var(--color-node-workflow) 15%, transparent);
}
.zoom-type--prompt { background: color-mix(in srgb, var(--color-node-prompt) 15%, transparent); }
.zoom-type--sequentialThinking { background: color-mix(in srgb, var(--color-node-thinking) 15%, transparent); }
.zoom-type--tool { background: color-mix(in srgb, var(--color-node-tool) 15%, transparent); }
.zoom-status {
  font-size: var(--text-xs); padding: 2px 6px; border-radius: 999px;
  color: var(--text-secondary);
  background: color-mix(in srgb, var(--color-status-pending) 15%, transparent);
}
.zoom-status--active { background: color-mix(in srgb, var(--color-status-active) 15%, transparent); color: var(--color-status-active); }
.zoom-status--completed { background: color-mix(in srgb, var(--color-status-completed) 15%, transparent); color: var(--color-status-completed); }
.zoom-status--abandoned { background: color-mix(in srgb, var(--color-status-abandoned) 15%, transparent); color: var(--color-status-abandoned); }
.zoom-status--ready { background: color-mix(in srgb, var(--color-status-ready) 15%, transparent); color: var(--color-status-ready); }
.zoom-close {
  background: none; border: none; color: var(--text-muted);
  font-size: var(--text-xl); cursor: pointer; padding: var(--space-1);
  line-height: 1; border-radius: var(--radius-sm);
}
.zoom-close:hover { color: var(--text-primary); background: var(--bg-surface); }
.zoom-close:focus-visible { outline: 2px solid var(--border-focus); outline-offset: 2px; }
.zoom-body {
  flex: 1; overflow-y: auto; padding: var(--space-4);
  display: flex; flex-direction: column; gap: var(--space-3);
}
.zoom-section-title {
  font-family: var(--font-headline, var(--font-sans));
  font-size: var(--text-sm); font-weight: 600; color: var(--text-primary);
  cursor: pointer; padding: var(--space-2) 0; list-style: none;
}
.zoom-section-title::before { content: '▸ '; }
.zoom-section[open] > .zoom-section-title::before { content: '▾ '; }
.zoom-pre {
  margin: var(--space-2) 0 0; padding: var(--space-3);
  font-family: var(--font-mono); font-size: var(--text-xs); line-height: 1.5;
  color: var(--text-secondary); background: var(--bg-main);
  border-radius: var(--radius-md); white-space: pre-wrap; word-break: break-word;
  max-height: 300px; overflow-y: auto;
}
.zoom-list { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: var(--space-2); }
.zoom-list-item {
  display: flex; flex-direction: column; gap: var(--space-1);
  padding: var(--space-2); background: var(--bg-surface); border-radius: var(--radius-md);
}
.item-label { font-family: var(--font-mono); font-size: var(--text-xs); font-weight: 600; color: var(--text-primary); }
.item-status { font-size: var(--text-xs); color: var(--text-muted); }
.zoom-empty { padding: var(--space-8); text-align: center; color: var(--text-muted); font-size: var(--text-sm); }
</style>
