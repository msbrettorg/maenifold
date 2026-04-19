<script setup lang="ts">
import { Brain, MessageSquare, Settings, Wrench, Palette, Lock } from 'lucide-vue-next';
import { computed } from 'vue';
import type { Component } from 'vue';
import PipelineNodeCardPorts from './PipelineNodeCardPorts.vue';
import PipelineNodeCardStatus from './PipelineNodeCardStatus.vue';

import type {
  NodeStatus,
  PipelineNode,
  PipelineNodeAdopt,
  PipelineStatus,
} from '../lib/pipeline-types';
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
  adopt?: PipelineNodeAdopt;
  requiresRole?: string;
  status?: NodeStatus | PipelineStatus | 'running';
  attempts?: number;
  type?: PipelineNode['type'];
  prompt?: string;
  agent?: string;
  topic?: string;
  totalThoughts?: number;
  thinkingTraceLength?: number;
  analysisType?: string;
  toolName?: string;
  toolArgs?: Record<string, unknown>;
  failureReason?: string | null;
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
      ...(data.adopt ? { adopt: data.adopt } : {}),
      ...(data.requiresRole ? { requiresRole: data.requiresRole } : {}),
      ...(data.prompt ? { prompt: data.prompt } : {}),
    };
  }

  if (data.type === 'prompt' || data.workflowId?.startsWith(promptPrefix)) {
    return {
      id: data.nodeId,
      type: 'prompt',
      agent: data.agent ?? data.acpClient ?? 'agent',
      prompt: (data.prompt ?? preview) || data.displayLabel || data.nodeId,
      ...(data.adopt ? { adopt: data.adopt } : {}),
      ...(data.requiresRole ? { requiresRole: data.requiresRole } : {}),
    };
  }

  if (data.type === 'sequentialThinking' || data.topic || data.workflowId?.startsWith(thinkingPrefix)) {
    return {
      id: data.nodeId,
      type: 'sequentialThinking',
      topic: (data.topic ?? topic) || data.displayLabel || data.nodeId,
      agent: data.agent ?? data.acpClient ?? 'agent',
      ...(data.adopt ? { adopt: data.adopt } : {}),
      ...(data.requiresRole ? { requiresRole: data.requiresRole } : {}),
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
  if (nodeType.value === 'tool') return 'var(--color-node-tool)';
  return 'var(--color-node-thinking)';
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
const nodeIcon = computed((): Component => {
  if (isToolNode(node.value)) return Wrench;
  if (isPromptNode(node.value)) return MessageSquare;
  if (isSequentialThinkingNode(node.value)) return Brain;
  return Settings;
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
const thinkingTraceLength = computed(() =>
  typeof props.data.thinkingTraceLength === 'number' ? props.data.thinkingTraceLength : 0,
);
const thoughtProgress = computed(() => {
  if (!isSequentialThinkingNode(node.value) || typeof node.value.totalThoughts !== 'number') {
    return null;
  }
  return `Thought ${thinkingTraceLength.value} of ${node.value.totalThoughts}`;
});
const promptPreview = computed(() => isPromptNode(node.value) ? shorten(node.value.prompt, 120) : '');
const workflowPreview = computed(() => isWorkflowNode(node.value) && node.value.prompt ? shorten(node.value.prompt, 120) : '');
const adoptSummary = computed(() => {
  if (isToolNode(node.value) || !node.value.adopt) {
    return [];
  }

  const summary: string[] = [];
  if (node.value.adopt.role) summary.push(`role: ${node.value.adopt.role}`);
  if (node.value.adopt.color) summary.push(`color: ${node.value.adopt.color}`);
  if (node.value.adopt.perspective) summary.push(`perspective: ${node.value.adopt.perspective}`);
  return summary;
});
const requiresRoleBadge = computed(() => {
  if (isToolNode(node.value)) {
    return '';
  }
  return node.value.requiresRole?.trim() ?? '';
});
const failureSummary = computed(() => {
  const failureReason = props.data.failureReason?.trim();
  if (!failureReason) return null;
  return shorten(failureReason, 100);
});
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
        :failure-summary="failureSummary"
        :status-appearance="statusAppearance"
        :type-icon="nodeIcon"
        :type-label="typeLabel"
      />

      <div v-if="showSubtitle" class="node-subtitle">{{ titleText }}</div>

      <template v-if="isWorkflowNode(node)">
        <div class="meta-row">
          <span class="meta-pill">workflow: {{ node.workflowId }}</span>
          <span v-if="node.acpClient" class="meta-pill">{{ node.acpClient }}</span>
          <span v-for="adoptValue in adoptSummary" :key="`workflow-adopt-${adoptValue}`" class="meta-pill"><Palette class="inline size-3" :stroke-width="1.5" /> {{ adoptValue }}</span>
          <span v-if="requiresRoleBadge" class="meta-pill"><Lock class="inline size-3" :stroke-width="1.5" /> requires: {{ requiresRoleBadge }}</span>
          <span v-if="modelBadge" class="meta-pill">model: {{ modelBadge }}</span>
          <span v-if="attempts" class="meta-pill">{{ attempts }}</span>
        </div>
        <div v-if="workflowPreview" class="preview">{{ workflowPreview }}</div>
      </template>

      <template v-else-if="isPromptNode(node)">
        <div class="meta-row">
          <span class="meta-pill">{{ node.agent }}</span>
          <span v-for="adoptValue in adoptSummary" :key="`prompt-adopt-${adoptValue}`" class="meta-pill"><Palette class="inline size-3" :stroke-width="1.5" /> {{ adoptValue }}</span>
          <span v-if="requiresRoleBadge" class="meta-pill"><Lock class="inline size-3" :stroke-width="1.5" /> requires: {{ requiresRoleBadge }}</span>
          <span v-if="isActive" class="meta-pill working-pill">
            <span class="working-spinner" aria-hidden="true" />
            Working...
          </span>
          <span v-if="modelBadge" class="meta-pill">model: {{ modelBadge }}</span>
          <span v-if="attempts" class="meta-pill">{{ attempts }}</span>
        </div>
        <div class="preview preview-code">{{ promptPreview }}</div>
      </template>

      <template v-else-if="isToolNode(node)">
        <div class="meta-row">
          <span class="meta-pill">{{ toolCategory }}: {{ node.toolName }}</span>
          <span v-if="modelBadge" class="meta-pill">model: {{ modelBadge }}</span>
          <span v-if="attempts" class="meta-pill">{{ attempts }}</span>
        </div>
      </template>

      <template v-else-if="isSequentialThinkingNode(node)">
        <div class="meta-row">
          <span class="meta-pill">{{ node.agent }}</span>
          <span v-for="adoptValue in adoptSummary" :key="`thinking-adopt-${adoptValue}`" class="meta-pill"><Palette class="inline size-3" :stroke-width="1.5" /> {{ adoptValue }}</span>
          <span v-if="requiresRoleBadge" class="meta-pill"><Lock class="inline size-3" :stroke-width="1.5" /> requires: {{ requiresRoleBadge }}</span>
          <span v-if="modelBadge" class="meta-pill">model: {{ modelBadge }}</span>
          <span class="meta-pill">topic: {{ node.topic }}</span>
          <span v-if="thoughtProgress" class="meta-pill">{{ thoughtProgress }}</span>
          <span v-if="node.analysisType" class="meta-pill">{{ node.analysisType }}</span>
          <span v-if="attempts" class="meta-pill">{{ attempts }}</span>
        </div>
      </template>
    </div>

    <PipelineNodeCardPorts />
  </div>
</template>
