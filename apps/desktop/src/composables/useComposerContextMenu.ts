import { computed, ref, watch } from 'vue';

import { useMcpToolDiscovery } from './useMcpToolDiscovery';
import type { PipelineNode, PipelineView } from '../lib/pipeline-types';
import { createNode, createNodeId } from '../lib/pipeline-node-factory';
import { getBareToolName } from '../lib/tool-categories';
import { resolvePreferredAgent } from '../lib/preferred-agent';
import { useConfigStore } from '../stores/config';
import { usePipelineStore } from '../stores/pipeline';

type Point = { x: number; y: number };

type ContextMenuState =
  | { mode: 'canvas'; position: Point }
  | { mode: 'node'; nodeId: string };

type UseComposerContextMenuOptions = {
  designMode: () => boolean;
  view: () => PipelineView | null | undefined;
  toFlowPosition: (clientX: number, clientY: number, rootElement: HTMLElement) => Point;
  createNodeAtPosition: (node: PipelineNode, position: Point) => void;
  editNode: (nodeId: string) => void;
  duplicateNode: (nodeId: string) => void;
  deleteNode: (nodeId: string) => void;
  disconnectNodeEdges: (nodeId: string) => void;
  emitNodeDoubleClick: (nodeId: string) => void;
  emitOpenGraft: () => void;
  emitOpenPromotion: () => void;
};

export function useComposerContextMenu(options: UseComposerContextMenuOptions) {
  const store = usePipelineStore();
  const configStore = useConfigStore();
  const { serverResults, hasLoadedCatalog, refreshTools } = useMcpToolDiscovery();
  const contextMenu = ref<ContextMenuState | null>(null);

  const firstAgent = computed(() => resolvePreferredAgent(configStore.agentNames));
  const firstWorkflow = computed(() => store.workflows[0] ?? '');
  const toolGroups = computed(() =>
    serverResults.value
      .map((result) => ({
        server: result.server,
        tools: [...result.tools]
          .sort((left, right) => left.name.localeCompare(right.name))
          .map((tool) => ({
            name: tool.name,
            displayName: tool.name.startsWith(`${result.server}.`)
              ? tool.name.slice(result.server.length + 1)
              : getBareToolName(tool.name),
            description: tool.description,
          })),
      }))
      .filter((group) => group.tools.length > 0),
  );

  const currentPipelineStatus = computed(() => options.view()?.status ?? null);
  const currentPipelineRunning = computed(() => options.view()?.running ?? false);
  const hasRewindableNodes = computed(() =>
    options.view()?.nodes.some((node) => ['completed', 'abandoned', 'stuck'].includes(node.status)) ?? false,
  );
  const hasCompletedNodesWithOutput = computed(() =>
    options.view()?.nodes.some((node) => node.status === 'completed' && Boolean(node.output)) ?? false,
  );
  const contextMenuNodeStatus = computed(() => {
    if (!contextMenu.value || contextMenu.value.mode !== 'node') return null;
    const nodeId = contextMenu.value.nodeId;
    return options.view()?.nodes.find((node) => node.nodeId === nodeId)?.status ?? null;
  });
  const contextMenuNodeHasOutput = computed(() => {
    if (!contextMenu.value || contextMenu.value.mode !== 'node') return false;
    const nodeId = contextMenu.value.nodeId;
    return Boolean(options.view()?.nodes.find((node) => node.nodeId === nodeId)?.output);
  });

  function hasServerCanvasMenuActions(): boolean {
    const view = options.view();
    if (!view) return false;

    return (
      ['active', 'paused', 'stuck'].includes(view.status)
      || view.nodes.some((node) => ['completed', 'abandoned', 'stuck'].includes(node.status))
      || view.nodes.some((node) => node.status === 'completed' && Boolean(node.output))
    );
  }

  function closeContextMenu(): void {
    contextMenu.value = null;
  }

  function createContextNode(type: 'prompt' | 'sequentialThinking' | 'workflow' | 'tool', toolName?: string): void {
    if (!contextMenu.value || contextMenu.value.mode !== 'canvas') return;

    const node = type === 'prompt'
      ? firstAgent.value
        ? createNode('prompt', { id: createNodeId('prompt'), agent: firstAgent.value, prompt: 'Ask an agent' })
        : null
      : type === 'sequentialThinking'
        ? firstAgent.value
          ? createNode('sequentialThinking', {
              id: createNodeId('sequentialThinking'),
              agent: firstAgent.value,
              topic: 'Think through a problem',
              totalThoughts: 5,
            })
          : null
        : type === 'workflow'
          ? firstWorkflow.value
            ? createNode('workflow', { id: createNodeId('workflow'), workflowId: firstWorkflow.value, agent: '' })
            : null
          : toolName
            ? createNode('tool', { id: createNodeId('tool'), toolName })
            : null;

    if (!node) return;
    options.createNodeAtPosition(node, contextMenu.value.position);
    closeContextMenu();
  }

  function maybeRefreshComposerCatalogs(): void {
    if (!options.designMode()) return;
    if (store.workflows.length === 0) void store.refreshWorkflows();
    if (!hasLoadedCatalog.value) void refreshTools();
  }

  function handleComposerContextMenu(event: MouseEvent): void {
    const target = event.target;
    if (!(target instanceof Element)) return;
    if (target.closest('.vue-flow__controls, .vue-flow__minimap')) return;

    const flowRoot = event.currentTarget instanceof HTMLElement
      ? event.currentTarget.querySelector<HTMLElement>('.vue-flow')
      : target.closest<HTMLElement>('.vue-flow');
    if (!flowRoot) {
      closeContextMenu();
      return;
    }

    const nodeId = target.closest<HTMLElement>('.vue-flow__node')?.dataset.id;
    if (nodeId) {
      options.editNode(nodeId);
      contextMenu.value = { mode: 'node', nodeId };
      return;
    }

    if (target.closest('.vue-flow__edge, .edge-indicator')) {
      closeContextMenu();
      return;
    }

    if (!options.designMode() && !hasServerCanvasMenuActions()) {
      closeContextMenu();
      return;
    }

    contextMenu.value = {
      mode: 'canvas',
      position: options.toFlowPosition(event.clientX, event.clientY, flowRoot),
    };
  }

  function handleEditNode(): void {
    if (!contextMenu.value || contextMenu.value.mode !== 'node') return;
    options.emitNodeDoubleClick(contextMenu.value.nodeId);
    closeContextMenu();
  }

  function handleDuplicateNode(): void {
    if (!contextMenu.value || contextMenu.value.mode !== 'node') return;
    options.duplicateNode(contextMenu.value.nodeId);
    closeContextMenu();
  }

  function handleDeleteNode(): void {
    if (!contextMenu.value || contextMenu.value.mode !== 'node') return;
    options.deleteNode(contextMenu.value.nodeId);
    closeContextMenu();
  }

  function handleDisconnectEdges(): void {
    if (!contextMenu.value || contextMenu.value.mode !== 'node') return;
    options.disconnectNodeEdges(contextMenu.value.nodeId);
    closeContextMenu();
  }

  function handleServerNodeAction(action: 'restartNode' | 'rewindNode' | 'forkNode' | 'abandonNode'): void {
    if (!contextMenu.value || contextMenu.value.mode !== 'node') return;
    const { nodeId } = contextMenu.value;
    closeContextMenu();

    switch (action) {
      case 'restartNode':
        void store.restartNode(nodeId);
        break;
      case 'rewindNode':
        void store.rewindNodes([nodeId]);
        break;
      case 'forkNode': {
        if (!store.currentPipelineId) break;
        const requestedName = window.prompt('Optional name for the forked pipeline session:', '');
        if (requestedName === null) break;
        void store.forkPipeline(store.currentPipelineId, nodeId, requestedName.trim() || undefined);
        break;
      }
      case 'abandonNode':
        void store.updateNodeStatus(nodeId, 'abandoned');
        break;
    }
  }

  function handleCanvasRunPipeline(): void {
    void store.runPipeline();
    closeContextMenu();
  }

  function handleCanvasRewindAll(): void {
    const rewindableIds = options.view()?.nodes
      .filter((node) => ['completed', 'abandoned', 'stuck'].includes(node.status))
      .map((node) => node.nodeId) ?? [];
    if (rewindableIds.length > 0) void store.rewindNodes(rewindableIds);
    closeContextMenu();
  }

  function handleCanvasGraft(): void {
    options.emitOpenGraft();
    closeContextMenu();
  }

  function handleCanvasPromoteToMemory(): void {
    options.emitOpenPromotion();
    closeContextMenu();
  }

  watch(() => options.designMode(), maybeRefreshComposerCatalogs, { immediate: true });

  return {
    contextMenu,
    closeContextMenu,
    contextMenuNodeHasOutput,
    contextMenuNodeStatus,
    createPromptNode: () => createContextNode('prompt'),
    createThinkingNode: () => createContextNode('sequentialThinking'),
    createToolNode: (toolName: string) => createContextNode('tool', toolName),
    createWorkflowNode: () => createContextNode('workflow'),
    currentPipelineRunning,
    currentPipelineStatus,
    firstAgent,
    firstWorkflow,
    handleCanvasGraft,
    handleCanvasPromoteToMemory,
    handleCanvasRewindAll,
    handleCanvasRunPipeline,
    handleComposerContextMenu,
    handleDeleteNode,
    handleDisconnectEdges,
    handleDuplicateNode,
    handleEditNode,
    handleServerNodeAction,
    hasCompletedNodesWithOutput,
    hasRewindableNodes,
    toolGroups,
  };
}
