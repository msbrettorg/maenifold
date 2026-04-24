<script setup lang="ts">
import { computed } from 'vue';
import { ContextMenuContent } from '@/components/ui/context-menu';
import PipelineComposerContextMenuDesign from './PipelineComposerContextMenuDesign.vue';
import PipelineComposerContextMenuServer from './PipelineComposerContextMenuServer.vue';

interface ToolMenuItem {
  name: string;
  displayName: string;
  description?: string | null;
}

interface ToolMenuGroup {
  server: string;
  tools: ToolMenuItem[];
}

const props = defineProps<{
  mode: 'design' | 'server';
  target: 'canvas' | 'node';
  toolGroups: ToolMenuGroup[];
  canAskAgent: boolean;
  canThink: boolean;
  canRunWorkflow: boolean;
  pipelineStatus?: string | null;
  pipelineRunning?: boolean;
  /** Canvas-level: pipeline has completed/abandoned/stuck nodes. */
  hasRewindableNodes?: boolean;
  /** Canvas-level: pipeline has completed nodes with non-empty output. */
  hasCompletedNodesWithOutput?: boolean;
  /** Node-level: status of the right-clicked node (server mode). */
  nodeStatus?: string | null;
  /** Node-level: whether the right-clicked node has output (server mode). */
  nodeHasOutput?: boolean;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'ask-agent'): void;
  (e: 'add-session-node'): void;
  (e: 'think-problem'): void;
  (e: 'run-workflow'): void;
  (e: 'run-tool', toolName: string): void;
  (e: 'edit'): void;
  (e: 'duplicate'): void;
  (e: 'delete'): void;
  (e: 'disconnect-edges'): void;
  (e: 'restart'): void;
  (e: 'rewind'): void;
  (e: 'fork'): void;
  (e: 'abandon'): void;
  (e: 'run-pipeline'): void;
  (e: 'rewind-all'): void;
  (e: 'graft'): void;
  (e: 'promote-to-memory'): void;
}>();

const isNonTerminal = computed(() => {
  const s = props.pipelineStatus;
  return s === 'active' || s === 'paused' || s === 'stuck';
});

const hasServerCanvasActions = computed(() => {
  if (props.mode !== 'server' || props.target !== 'canvas') return false;
  return isNonTerminal.value || Boolean(props.hasRewindableNodes) || Boolean(props.hasCompletedNodesWithOutput);
});

// Node-level action visibility (server mode, target === 'node').
// When nodeStatus is unknown/null, fall back to showing all items.
const showRestart = computed(() => {
  const s = props.nodeStatus;
  if (!s) return true;
  return s === 'completed' || s === 'abandoned' || s === 'stuck';
});

const showRewind = computed(() => {
  const s = props.nodeStatus;
  if (!s) return true;
  return s === 'completed' || s === 'abandoned' || s === 'stuck';
});

const showFork = computed(() => {
  const s = props.nodeStatus;
  if (!s) return true;
  return s === 'completed' && Boolean(props.nodeHasOutput);
});

const showAbandon = computed(() => {
  const s = props.nodeStatus;
  if (!s) return true;
  return s === 'pending' || s === 'ready' || s === 'active' || s === 'retrying' || s === 'stuck';
});
</script>

<template>
  <ContextMenuContent
    v-if="mode === 'design' || target === 'node' || hasServerCanvasActions"
    class="min-w-[220px]"
    :side-offset="8"
    align="start"
    @close-auto-focus.prevent
  >
    <PipelineComposerContextMenuDesign
      v-if="mode === 'design'"
      :target="target"
      :tool-groups="toolGroups"
      :can-ask-agent="canAskAgent"
      :can-think="canThink"
      :can-run-workflow="canRunWorkflow"
      @close="emit('close')"
      @ask-agent="emit('ask-agent')"
      @add-session-node="emit('add-session-node')"
      @think-problem="emit('think-problem')"
      @run-workflow="emit('run-workflow')"
      @run-tool="emit('run-tool', $event)"
      @edit="emit('edit')"
      @duplicate="emit('duplicate')"
      @delete="emit('delete')"
      @disconnect-edges="emit('disconnect-edges')"
    />

    <PipelineComposerContextMenuServer
      v-else-if="mode === 'server'"
      :target="target"
      :pipeline-status="pipelineStatus"
      :pipeline-running="pipelineRunning"
      :has-rewindable-nodes="hasRewindableNodes"
      :has-completed-nodes-with-output="hasCompletedNodesWithOutput"
      :is-non-terminal="isNonTerminal"
      :show-restart="showRestart"
      :show-rewind="showRewind"
      :show-fork="showFork"
      :show-abandon="showAbandon"
      @close="emit('close')"
      @restart="emit('restart')"
      @rewind="emit('rewind')"
      @fork="emit('fork')"
      @abandon="emit('abandon')"
      @run-pipeline="emit('run-pipeline')"
      @rewind-all="emit('rewind-all')"
      @graft="emit('graft')"
      @promote-to-memory="emit('promote-to-memory')"
    />
  </ContextMenuContent>
</template>
