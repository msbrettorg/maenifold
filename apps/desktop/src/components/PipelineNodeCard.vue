<script setup lang="ts">
import { computed } from 'vue';
import PipelineNodeCardPorts from './PipelineNodeCardPorts.vue';
import PipelineNodeCardStatus from './PipelineNodeCardStatus.vue';

import type { NodeStatus, PipelineNode, PipelineStatus } from '../lib/pipeline-types';
import { normalizeNodeStatusAppearance } from '../lib/pipeline-node-status';
import {
  getNodeType,
  isPromptNode,
  isSequentialThinkingNode,
  isToolNode,
  isWorkflowNode,
  nodeLabel,
} from '../lib/pipeline-types';
import { getToolCategory } from '../lib/tool-categories';

type NodeCardData = {
  nodeId: string;
  displayLabel?: string;
  model?: string;
  workflowId?: string;
  acpClient?: string;
  status?: NodeStatus | PipelineStatus | 'running';
  attempts?: number;
  type?: PipelineNode['type'];
  prompt?: string;
  agent?: string;
  topic?: string;
  totalThoughts?: number;
  analysisType?: string;
  toolName?: string;
  toolArgs?: Record<string, unknown>;
};

const props = defineProps<{ id: string; data: NodeCardData; selected?: boolean }>();
const promptPrefix = 'prompt: ';
const thinkingPrefix = 'think: ';

const shorten = (text: string, max = 48) =>
  text.length > max ? `${text.slice(0, max).trimEnd()}...` : text;
const stripPrefix = (value: string | undefined, prefix: string) =>
  value?.startsWith(prefix) ? value.slice(prefix.length) : (value ?? '');
const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === 'object' && value !== null && !Array.isArray(value);

function inferNode(data: NodeCardData): PipelineNode {
  const preview = stripPrefix(data.workflowId, promptPrefix);
  const topic = stripPrefix(data.workflowId, thinkingPrefix);

  // Explicit type wins — check workflow first to avoid heuristic misclassification
  if (data.type === 'workflow' || (data.workflowId && !data.type)) {
    return {
      id: data.nodeId,
      type: 'workflow',
      workflowId: data.workflowId ?? data.nodeId,
      ...(data.acpClient ? { acpClient: data.acpClient } : {}),
      ...(data.prompt ? { prompt: data.prompt } : {}),
    };
  }

  if (data.type === 'prompt' || data.workflowId?.startsWith(promptPrefix)) {
    return {
      id: data.nodeId,
      type: 'prompt',
      agent: data.agent ?? data.acpClient ?? 'agent',
      prompt: (data.prompt ?? preview) || data.displayLabel || data.nodeId,
    };
  }

  if (data.type === 'sequentialThinking' || data.topic || data.workflowId?.startsWith(thinkingPrefix)) {
    return {
      id: data.nodeId,
      type: 'sequentialThinking',
      topic: (data.topic ?? topic) || data.displayLabel || data.nodeId,
      agent: data.agent ?? data.acpClient ?? 'agent',
      ...(typeof data.totalThoughts === 'number' ? { totalThoughts: data.totalThoughts } : {}),
      ...(data.analysisType ? { analysisType: data.analysisType } : {}),
    };
  }

  if (data.type === 'tool' || data.toolName) {
    return {
      id: data.nodeId,
      type: 'tool',
      toolName: data.toolName ?? data.displayLabel ?? data.nodeId,
      ...(data.agent ? { agent: data.agent } : {}),
      ...(isRecord(data.toolArgs) ? { toolArgs: data.toolArgs } : {}),
    };
  }

  return {
    id: data.nodeId,
    type: 'workflow',
    workflowId: data.workflowId ?? data.displayLabel ?? data.nodeId,
    ...(data.acpClient ? { acpClient: data.acpClient } : {}),
    ...(data.prompt ? { prompt: data.prompt } : {}),
  };
}

const node = computed(() => inferNode(props.data));
const nodeType = computed(() => getNodeType(node.value));
const toolCategory = computed(() =>
  isToolNode(node.value) ? (getToolCategory(node.value.toolName)?.label ?? 'Tool') : null,
);
const accentColor = computed(() => {
  if (nodeType.value === 'prompt') return 'var(--color-node-prompt)';
  if (nodeType.value === 'sequentialThinking') return 'var(--color-node-thinking)';
  if (nodeType.value === 'tool') {
    switch (toolCategory.value) {
      case 'Memory': return 'var(--color-node-tool)';
      case 'Graph': return 'var(--color-node-tool)';
      case 'Workflow': return 'var(--color-node-workflow)';
      case 'System': return 'var(--color-node-tool)';
      default: return 'var(--color-node-tool)';
    }
  }
  return 'var(--color-node-workflow)';
});
const typeLabel = computed(() =>
  isPromptNode(node.value)
    ? 'prompt'
    : isSequentialThinkingNode(node.value)
      ? 'sequential thinking'
      : isToolNode(node.value)
        ? (toolCategory.value?.toLowerCase() ?? 'tool')
      : 'workflow',
);
const typeIcon = computed(() => {
  if (isToolNode(node.value)) return getToolCategory(node.value.toolName)?.icon ?? '🔧';
  if (isPromptNode(node.value)) return '💬';
  if (isSequentialThinkingNode(node.value)) return '🧠';
  return '⚙️';
});
const displayLabel = computed(() => props.data.displayLabel || props.data.nodeId);
const titleText = computed(() => shorten(nodeLabel(node.value), 44));
const hasCustomLabel = computed(() => Boolean(props.data.displayLabel && props.data.displayLabel !== props.data.nodeId));
const showSubtitle = computed(() => !hasCustomLabel.value || titleText.value !== displayLabel.value);
const currentStatus = computed(() => props.data.status ?? 'pending');
const statusAppearance = computed(() => normalizeNodeStatusAppearance(currentStatus.value));
const nodeStatusClass = computed(() => `node-status-${statusAppearance.value}`);
const isActive = computed(() => statusAppearance.value === 'active');
const ariaLabel = computed(() => `${typeLabel.value} node: ${displayLabel.value}`);
const attempts = computed(() => props.data.attempts && props.data.attempts > 1 ? `×${props.data.attempts}` : '');
const modelBadge = computed(() => props.data.model?.trim() ?? '');
const promptPreview = computed(() => isPromptNode(node.value) ? shorten(node.value.prompt, 120) : '');
const workflowPreview = computed(() => isWorkflowNode(node.value) && node.value.prompt ? shorten(node.value.prompt, 120) : '');
</script>

<template>
  <div
    class="pipeline-node"
    :class="[nodeStatusClass, { 'is-active': isActive }]"
    :style="{ '--node-accent': accentColor }"
    :title="`${titleText} • ${data.status ?? 'pending'}`"
    role="button"
    tabindex="0"
    :aria-label="ariaLabel"
  >
    <div class="accent-bar" />
    <div class="node-content">
      <PipelineNodeCardStatus
        :display-label="displayLabel"
        :status-appearance="statusAppearance"
        :type-icon="typeIcon"
        :type-label="typeLabel"
      />

      <div v-if="showSubtitle" class="node-subtitle">{{ titleText }}</div>

      <template v-if="isWorkflowNode(node)">
        <div class="meta-row">
          <span class="meta-pill">workflow: {{ node.workflowId }}</span>
          <span v-if="node.acpClient" class="meta-pill">{{ node.acpClient }}</span>
          <span v-if="modelBadge" class="meta-pill">model: {{ modelBadge }}</span>
          <span
            v-if="attempts"
            :class="['meta-pill', 'meta-pill-status', `status-dot--${statusAppearance}`]"
          >
            {{ attempts }}
          </span>
        </div>
        <div v-if="workflowPreview" class="preview">{{ workflowPreview }}</div>
      </template>

      <template v-else-if="isPromptNode(node)">
        <div class="meta-row">
          <span class="meta-pill">{{ node.agent }}</span>
          <span v-if="modelBadge" class="meta-pill">model: {{ modelBadge }}</span>
          <span
            v-if="attempts"
            :class="['meta-pill', 'meta-pill-status', `status-dot--${statusAppearance}`]"
          >
            {{ attempts }}
          </span>
        </div>
        <div class="preview preview-code">{{ promptPreview }}</div>
      </template>

      <template v-else-if="isToolNode(node)">
        <div class="meta-row">
          <span class="meta-pill">{{ toolCategory }}: {{ node.toolName }}</span>
          <span v-if="modelBadge" class="meta-pill">model: {{ modelBadge }}</span>
          <span
            v-if="attempts"
            :class="['meta-pill', 'meta-pill-status', `status-dot--${statusAppearance}`]"
          >
            {{ attempts }}
          </span>
        </div>
      </template>

      <template v-else-if="isSequentialThinkingNode(node)">
        <div class="meta-row">
          <span class="meta-pill">{{ node.agent }}</span>
          <span v-if="modelBadge" class="meta-pill">model: {{ modelBadge }}</span>
          <span class="meta-pill">topic: {{ node.topic }}</span>
          <span v-if="typeof node.totalThoughts === 'number'" class="meta-pill">thoughts: {{ node.totalThoughts }}</span>
          <span v-if="node.analysisType" class="meta-pill">{{ node.analysisType }}</span>
          <span
            v-if="attempts"
            :class="['meta-pill', 'meta-pill-status', `status-dot--${statusAppearance}`]"
          >
            {{ attempts }}
          </span>
        </div>
      </template>
    </div>

    <PipelineNodeCardPorts />
  </div>
</template>

<style scoped>
.pipeline-node {
  position: relative; display: flex; min-width: 200px; max-width: 280px; max-height: 200px; overflow: hidden;
  background: var(--accent);
  border: 1px solid var(--node-status-border, #4b5563);
  border-radius: 8px; overflow: visible;
  transition: border-color .2s var(--easing-standard), box-shadow .2s var(--easing-standard), transform .2s var(--easing-standard);
  font-family: var(--font-body);
}
.pipeline-node:hover {
  background: var(--muted);
  transform: translateY(-1px);
}
.pipeline-node.is-active {
  animation: node-pulse 2s ease-in-out infinite, pulse-border 2s ease-in-out infinite;
}
.node-status-pending { --node-status-border: #4b5563; }
.node-status-active { --node-status-border: #3b82f6; }
.node-status-completed { --node-status-border: #22c55e; }
.node-status-abandoned { --node-status-border: #ef4444; }
.node-status-stuck { --node-status-border: #f59e0b; }
.node-status-cancelled { --node-status-border: #6b7280; border-style: dashed; }
.accent-bar { width: 4px; flex-shrink: 0; background: var(--node-accent); border-radius: 8px 0 0 8px; }
.node-content { flex: 1; min-width: 0; }
.node-title,.type-pill,.node-subtitle,.meta-pill,.preview { overflow: hidden; text-overflow: ellipsis; }
.type-pill,.meta-pill {
  display: inline-flex; align-items: center; min-height: 18px; padding: 2px 6px; border-radius: 999px;
  font-size: var(--text-xs); line-height: 1.1; white-space: nowrap;
}
.node-subtitle,.meta-row,.preview { padding-inline: 12px; }
.node-subtitle { margin: 8px 0 6px; font-size: var(--text-sm); color: var(--muted-foreground); white-space: nowrap; }
.meta-row { display: flex; flex-wrap: wrap; gap: 6px; padding-bottom: 8px; }
.meta-pill { max-width: 100%; color: var(--foreground); background: color-mix(in srgb, var(--node-accent) 8%, transparent); }
.meta-pill-status { color: var(--primary-foreground); }
.status-dot--pending { background: var(--color-status-pending); }
.status-dot--active { background: var(--color-status-active); }
.status-dot--completed { background: var(--color-status-completed); }
.status-dot--abandoned { background: var(--color-status-abandoned); }
.status-dot--stuck { background: var(--color-status-stuck); }
.status-dot--cancelled { background: #6b7280; }
.preview {
  margin: 0 12px 10px; padding: 8px 10px; font-size: var(--text-xs); line-height: 1.4; color: var(--muted-foreground);
  white-space: normal; word-break: break-word; background: color-mix(in srgb, var(--node-accent) 4%, transparent); border-radius: 6px;
  display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical;
}
.preview-code { font-family: var(--font-code); }
:deep(.vue-flow__node.pipeline-node-shell--active) {
  animation: pipeline-node-shell-pulse 2s ease-in-out infinite;
}

@keyframes node-pulse {
  0%, 100% {
    box-shadow: 0 0 20px color-mix(in srgb, var(--node-accent) 8%, transparent);
  }
  50% {
    box-shadow: 0 0 20px color-mix(in srgb, var(--node-accent) 18%, transparent);
  }
}

@keyframes pulse-border {
  0%, 100% { border-color: #3b82f6; }
  50% { border-color: #60a5fa; }
}

@keyframes pipeline-node-shell-pulse {
  0%, 100% { border-left-color: #3b82f6; }
  50% { border-left-color: #60a5fa; }
}
</style>
