<script setup lang="ts">
import { Brain, Loader2, Lock, MessageCircleMore, MessageSquare, Palette, RotateCcw, Settings, Wrench } from 'lucide-vue-next';
import { computed, ref } from 'vue';
import type { Component } from 'vue';

import PipelineNodeCardPorts from './PipelineNodeCardPorts.vue';
import PipelineNodeCardStatus from './PipelineNodeCardStatus.vue';
import { Card, CardContent } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';

import { normalizeNodeStatusAppearance } from '../lib/pipeline-node-status';
import {
  inferPipelineNodeFromCardData,
  shortenText,
  type NodeCardData,
} from '../lib/pipeline-node-card-data';
import {
  getNodeType,
  isPromptNode,
  isSequentialThinkingNode,
  isToolNode,
  isWorkflowNode,
  nodeLabel,
} from '../lib/pipeline-types-node';
import { getToolCategory } from '../lib/tool-categories';
import { usePipelineStore } from '../stores/pipeline';

const props = defineProps<{ id: string; data: NodeCardData; selected?: boolean }>();
const store = usePipelineStore();
const resumeBusy = ref(false);

const node = computed(() => inferPipelineNodeFromCardData(props.data));
const nodeType = computed(() => getNodeType(node.value));
const toolCategory = computed(() =>
  isToolNode(node.value) ? (getToolCategory(node.value.toolName)?.label ?? 'Tool') : null,
);
const isSessionNode = computed(
  () => isPromptNode(node.value) && Boolean(node.value.sessionRef?.sessionId),
);
const accentBarClass = computed(() => {
  if (nodeType.value === 'prompt') return 'bg-node-prompt';
  if (nodeType.value === 'tool') return 'bg-node-tool';
  if (nodeType.value === 'sequentialThinking') return 'bg-node-thinking';
  return 'bg-node-workflow';
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
  if (isPromptNode(node.value)) return node.value.sessionRef?.sessionId ? MessageCircleMore : MessageSquare;
  if (isSequentialThinkingNode(node.value)) return Brain;
  return Settings;
});
const displayLabel = computed(() => props.data.displayLabel || props.data.nodeId);
const titleText = computed(() => shortenText(nodeLabel(node.value), 44));
const hasCustomLabel = computed(
  () => Boolean(props.data.displayLabel && props.data.displayLabel !== props.data.nodeId),
);
const showSubtitle = computed(() => !hasCustomLabel.value || titleText.value !== displayLabel.value);
const currentStatus = computed(() => props.data.status ?? 'pending');
const statusAppearance = computed(() => normalizeNodeStatusAppearance(currentStatus.value));
const isActive = computed(() => statusAppearance.value === 'active');
const canResumeFromNode = computed(() => ['abandoned', 'stuck'].includes(currentStatus.value) && store.currentView?.running !== true);
const ariaLabel = computed(() => `${typeLabel.value} node: ${displayLabel.value}`);
const attempts = computed(() =>
  props.data.attempts && props.data.attempts > 1 ? `×${props.data.attempts}` : '',
);
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
const promptPreview = computed(() =>
  isPromptNode(node.value) ? shortenText(node.value.prompt, 120) : '',
);
const promptSessionBadge = computed(() => {
  if (!isPromptNode(node.value) || !node.value.sessionRef?.sessionId) {
    return '';
  }
  return shortenText(node.value.sessionRef.sessionId, 32);
});
const promptSessionTitle = computed(() => {
  if (!isPromptNode(node.value) || !node.value.sessionRef?.sessionId) {
    return '';
  }
  return `Session: ${node.value.sessionRef.sessionId}\nAgent: ${node.value.agent}`;
});
const workflowPreview = computed(() =>
  isWorkflowNode(node.value) && node.value.prompt ? shortenText(node.value.prompt, 120) : '',
);
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
  if (isToolNode(node.value)) return '';
  return node.value.requiresRole?.trim() ?? '';
});
const failureSummary = computed(() => {
  const failureReason = props.data.failureReason?.trim();
  if (!failureReason) return null;
  return shortenText(failureReason, 100);
});

async function resumeFromNode() {
  if (!canResumeFromNode.value || resumeBusy.value) return;
  resumeBusy.value = true;
  try {
    await store.resumeFromNode(props.data.nodeId);
  } finally {
    resumeBusy.value = false;
  }
}
</script>

<template>
  <div class="relative">
  <Card
    size="sm"
    :class="[
      'relative min-w-[260px] max-w-[340px] gap-0 rounded-xl py-0 transition-shadow',
      isSessionNode ? 'border-dashed border-node-prompt/40' : '',
      selected ? 'ring-2 ring-ring' : '',
      isActive ? 'shadow-lg' : '',
    ]"
    role="button"
    tabindex="0"
    :title="`${titleText} • ${data.status ?? 'pending'}`"
    :aria-label="ariaLabel"
  >
    <span
      :class="['absolute inset-y-0 left-0 w-1 rounded-l-xl', accentBarClass]"
      aria-hidden="true"
    />

    <PipelineNodeCardStatus
      :display-label="displayLabel"
      :failure-summary="failureSummary"
      :status-appearance="statusAppearance"
      :type-icon="nodeIcon"
      :type-label="typeLabel"
    />

    <CardContent class="flex flex-col gap-1.5 px-3 pb-2.5 pt-0">
      <div v-if="showSubtitle" class="truncate text-xs text-muted-foreground">
        {{ titleText }}
      </div>
      <Badge v-if="data.reconnected" variant="secondary" class="self-start font-normal">↻ refining</Badge>

      <template v-if="isWorkflowNode(node)">
        <div class="flex flex-wrap gap-1">
          <Badge variant="outline" class="font-normal">workflow: {{ node.workflowId }}</Badge>
          <Badge v-if="node.acpClient" variant="outline" class="font-normal">{{ node.acpClient }}</Badge>
          <Badge
            v-for="adoptValue in adoptSummary"
            :key="`workflow-adopt-${adoptValue}`"
            variant="outline"
            class="font-normal"
          >
            <Palette class="size-3" :stroke-width="1.5" />
            {{ adoptValue }}
          </Badge>
          <Badge v-if="requiresRoleBadge" variant="outline" class="font-normal">
            <Lock class="size-3" :stroke-width="1.5" />
            requires: {{ requiresRoleBadge }}
          </Badge>
          <Badge v-if="modelBadge" variant="outline" class="font-normal">model: {{ modelBadge }}</Badge>
          <Badge v-if="attempts" variant="outline" class="font-normal">{{ attempts }}</Badge>
        </div>
        <div
          v-if="workflowPreview"
          class="mt-1 line-clamp-3 whitespace-pre-wrap break-words text-xs text-muted-foreground"
        >
          {{ workflowPreview }}
        </div>
      </template>

      <template v-else-if="isPromptNode(node)">
        <div class="flex flex-wrap gap-1">
          <Badge variant="outline" class="font-normal">{{ node.agent }}</Badge>
          <Badge
            v-if="promptSessionBadge"
            variant="outline"
            class="font-normal"
            :title="promptSessionTitle"
          >
            <MessageCircleMore class="size-3" :stroke-width="1.5" />
            session: {{ promptSessionBadge }}
          </Badge>
          <Badge
            v-for="adoptValue in adoptSummary"
            :key="`prompt-adopt-${adoptValue}`"
            variant="outline"
            class="font-normal"
          >
            <Palette class="size-3" :stroke-width="1.5" />
            {{ adoptValue }}
          </Badge>
          <Badge v-if="requiresRoleBadge" variant="outline" class="font-normal">
            <Lock class="size-3" :stroke-width="1.5" />
            requires: {{ requiresRoleBadge }}
          </Badge>
          <Badge
            v-if="isActive"
            variant="secondary"
            class="font-normal"
          >
            <Loader2 class="size-3 animate-spin" :stroke-width="1.5" aria-hidden="true" />
            Working...
          </Badge>
          <Badge v-if="modelBadge" variant="outline" class="font-normal">model: {{ modelBadge }}</Badge>
          <Badge v-if="attempts" variant="outline" class="font-normal">{{ attempts }}</Badge>
        </div>
        <div
          class="mt-1 line-clamp-3 whitespace-pre-wrap break-words rounded-md bg-muted/50 px-2 py-1 font-mono text-xs text-muted-foreground"
        >
          {{ promptPreview }}
        </div>
      </template>

      <template v-else-if="isToolNode(node)">
        <div class="flex flex-wrap gap-1">
          <Badge variant="outline" class="font-normal">
            {{ toolCategory }}: {{ node.toolName }}
          </Badge>
          <Badge v-if="modelBadge" variant="outline" class="font-normal">model: {{ modelBadge }}</Badge>
          <Badge v-if="attempts" variant="outline" class="font-normal">{{ attempts }}</Badge>
        </div>
      </template>

      <template v-else-if="isSequentialThinkingNode(node)">
        <div class="flex flex-wrap gap-1">
          <Badge variant="outline" class="font-normal">{{ node.agent }}</Badge>
          <Badge
            v-for="adoptValue in adoptSummary"
            :key="`thinking-adopt-${adoptValue}`"
            variant="outline"
            class="font-normal"
          >
            <Palette class="size-3" :stroke-width="1.5" />
            {{ adoptValue }}
          </Badge>
          <Badge v-if="requiresRoleBadge" variant="outline" class="font-normal">
            <Lock class="size-3" :stroke-width="1.5" />
            requires: {{ requiresRoleBadge }}
          </Badge>
          <Badge v-if="modelBadge" variant="outline" class="font-normal">model: {{ modelBadge }}</Badge>
          <Badge variant="outline" class="font-normal">topic: {{ node.topic }}</Badge>
          <Badge v-if="thoughtProgress" variant="outline" class="font-normal">{{ thoughtProgress }}</Badge>
          <Badge v-if="node.analysisType" variant="outline" class="font-normal">{{ node.analysisType }}</Badge>
          <Badge v-if="attempts" variant="outline" class="font-normal">{{ attempts }}</Badge>
        </div>
      </template>

      <Button
        v-if="canResumeFromNode"
        type="button"
        size="xs"
        class="mt-1 self-start"
        :disabled="resumeBusy"
        aria-label="Resume pipeline from this node"
        @click.stop="resumeFromNode"
      >
        <RotateCcw class="size-3" :stroke-width="1.5" aria-hidden="true" />
        {{ resumeBusy ? 'Resuming...' : 'Resume from this node' }}
      </Button>
    </CardContent>
  </Card>
    <PipelineNodeCardPorts />
  </div>
</template>
