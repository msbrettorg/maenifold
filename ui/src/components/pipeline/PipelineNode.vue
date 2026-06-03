<script setup lang="ts">
// Custom Vue Flow node for all four pipeline node types.
//
// Visual encoding (carried from the old UI's blueprint, rebuilt in the current
// token language): a 4px left accent bar colored per node type, a status dot
// (pending / active-with-pulse / stuck / completed / abandoned), and — the
// improvement over the old UI — a celebratory "0 tokens" badge on every tool
// node (tool nodes run deterministically with no LLM inference).
import { computed } from 'vue';
import { Handle, Position } from '@vue-flow/core';
import type { NodeProps } from '@vue-flow/core';
import { Brain, MessageCircleMore, MessageSquare, Settings, Wrench } from '@lucide/vue';
import type { Component } from 'vue';
import type { FlowNodeData } from './canvasBridge';

const props = defineProps<NodeProps<FlowNodeData>>();

const icon = computed<Component>(() => {
  switch (props.data.nodeType) {
    case 'tool':
      return Wrench;
    case 'prompt':
      return props.data.sessionBound ? MessageCircleMore : MessageSquare;
    case 'sequentialThinking':
      return Brain;
    default:
      return Settings;
  }
});

const typeLabel = computed(() => {
  switch (props.data.nodeType) {
    case 'tool':
      return 'tool';
    case 'prompt':
      return 'prompt';
    case 'sequentialThinking':
      return 'thinking';
    default:
      return 'workflow';
  }
});

const attemptsBadge = computed(() => (props.data.attempts > 1 ? `×${props.data.attempts}` : ''));
const isActive = computed(() => props.data.statusKey === 'active');
</script>

<template>
  <div
    class="pnode"
    :class="[`pnode--${data.nodeType}`, `is-${data.statusKey}`, { selected, 'is-session': data.sessionBound }]"
    :title="`${data.label} • ${data.status}`"
  >
    <span class="pnode-accent" aria-hidden="true" />

    <!-- Both LR and TB handles are always present so edges can attach in either
         orientation without re-registering handles on a direction switch. -->
    <Handle id="in-left" type="target" :position="Position.Left" />
    <Handle id="in-top" type="target" :position="Position.Top" />
    <Handle id="out-right" type="source" :position="Position.Right" />
    <Handle id="out-bottom" type="source" :position="Position.Bottom" />

    <header class="pnode-head">
      <component :is="icon" class="pnode-icon" :size="15" :stroke-width="1.9" />
      <span class="pnode-label">{{ data.label }}</span>
      <span
        class="pnode-status"
        :class="{ pulse: isActive }"
        :aria-label="`status: ${data.status}`"
      />
    </header>

    <div class="pnode-meta">
      <span class="pnode-type">{{ typeLabel }}</span>
      <span v-if="data.subtitle" class="pnode-subtitle">{{ data.subtitle }}</span>
      <span v-if="attemptsBadge" class="pnode-attempts">{{ attemptsBadge }}</span>
    </div>

    <p v-if="data.detail" class="pnode-detail">{{ data.detail }}</p>

    <footer v-if="data.zeroTokens" class="pnode-foot">
      <span class="pnode-zero" title="Tool node — runs with zero LLM tokens">⚡ 0 tokens</span>
    </footer>
  </div>
</template>

<style scoped>
.pnode {
  position: relative;
  width: 264px;
  min-height: 92px;
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
  padding: 0.55rem 0.7rem 0.55rem 0.85rem;
  border: 1px solid var(--border-color);
  border-radius: 10px;
  background: color-mix(in srgb, var(--bg-main) 92%, var(--bg-sidebar));
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.08);
  color: var(--text-primary);
  overflow: hidden;
  transition: border-color 0.12s ease, box-shadow 0.12s ease;
}

.pnode.selected {
  border-color: color-mix(in srgb, var(--text-accent) 55%, var(--border-color));
  box-shadow: 0 0 0 2px color-mix(in srgb, var(--text-accent) 35%, transparent);
}

.pnode.is-session {
  border-style: dashed;
}

.pnode-accent {
  position: absolute;
  inset: 0 auto 0 0;
  width: 4px;
  border-radius: 10px 0 0 10px;
  background: var(--pnode-accent, var(--text-muted));
}

.pnode--prompt {
  --pnode-accent: var(--node-accent-prompt);
}
.pnode--workflow {
  --pnode-accent: var(--node-accent-workflow);
}
.pnode--sequentialThinking {
  --pnode-accent: var(--node-accent-thinking);
}
.pnode--tool {
  --pnode-accent: var(--node-accent-tool);
}

.pnode-head {
  display: flex;
  align-items: center;
  gap: 0.4rem;
}

.pnode-icon {
  flex-shrink: 0;
  color: var(--pnode-accent, var(--text-secondary));
}

.pnode-label {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 0.84rem;
  font-weight: 650;
}

.pnode-status {
  flex-shrink: 0;
  width: 9px;
  height: 9px;
  border-radius: 50%;
  background: var(--node-status-pending);
}

.is-active .pnode-status {
  background: var(--node-status-active);
}
.is-stuck .pnode-status {
  background: var(--node-status-stuck);
}
.is-completed .pnode-status {
  background: var(--node-status-completed);
}
.is-abandoned .pnode-status {
  background: var(--node-status-abandoned);
}

.pnode-status.pulse {
  box-shadow: 0 0 0 0 color-mix(in srgb, var(--node-status-active) 70%, transparent);
  animation: pnode-pulse 1.4s ease-out infinite;
}

@keyframes pnode-pulse {
  0% {
    box-shadow: 0 0 0 0 color-mix(in srgb, var(--node-status-active) 65%, transparent);
  }
  70% {
    box-shadow: 0 0 0 7px color-mix(in srgb, var(--node-status-active) 0%, transparent);
  }
  100% {
    box-shadow: 0 0 0 0 color-mix(in srgb, var(--node-status-active) 0%, transparent);
  }
}

.pnode-meta {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 0.3rem;
  font-size: 0.7rem;
}

.pnode-type {
  padding: 0.05rem 0.35rem;
  border-radius: 5px;
  background: color-mix(in srgb, var(--pnode-accent) 18%, transparent);
  color: color-mix(in srgb, var(--pnode-accent) 75%, var(--text-primary));
  text-transform: uppercase;
  letter-spacing: 0.02em;
  font-weight: 700;
}

.pnode-subtitle {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text-secondary);
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
}

.pnode-attempts {
  color: var(--node-status-stuck);
  font-variant-numeric: tabular-nums;
}

.pnode-detail {
  margin: 0;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  color: var(--text-muted);
  font-size: 0.72rem;
  line-height: 1.35;
}

.pnode-foot {
  display: flex;
}

.pnode-zero {
  display: inline-flex;
  align-items: center;
  gap: 0.2rem;
  padding: 0.1rem 0.4rem;
  border-radius: 999px;
  background: color-mix(in srgb, var(--node-accent-tool) 22%, transparent);
  color: color-mix(in srgb, var(--node-accent-tool) 80%, var(--text-primary));
  font-size: 0.68rem;
  font-weight: 700;
}
</style>
