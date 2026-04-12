import { computed, markRaw, nextTick, ref, watch } from 'vue';
import { getSmoothStepPath, useVueFlow } from '@vue-flow/core';
import type { Connection, Edge, EdgeProps, Node } from '@vue-flow/core';

import PipelineNodeCard from '../components/PipelineNodeCard.vue';
import { useComposerSync } from './useComposerSync';
import { useLayout } from './useLayout';
import {
  definitionFromFlow,
  type FlowNodeData,
  loadPositions,
  pipelineEdgesToFlow,
  pipelineNodesToFlow,
  savePositions,
  toPipelineNodeFromViewNode,
} from '../lib/pipeline-composer-flow';
import { createNode, createNodeId } from '../lib/pipeline-node-factory';
import type { PipelineDefinition, PipelineNode, PipelineView } from '../lib/pipeline-types';
import {
  isPromptNode,
  isSequentialThinkingNode,
  isToolNode,
  isWorkflowNode,
} from '../lib/pipeline-types';
import { useSemanticZoomStore } from '../stores/semantic-zoom';
import { useSessionStore } from '../stores/session';
import { useAppShellStore } from '../stores/app-shell';

type SelectedEdge = { source: string; target: string };

type UseComposerCanvasOptions = {
  designMode: () => boolean;
  localDefinition: () => PipelineDefinition | undefined;
  view: () => PipelineView | null | undefined;
  updateDefinition: (definition: PipelineDefinition) => void;
  selectNode: (nodeId: string | null) => void;
  selectEdge: (edge: SelectedEdge | null) => void;
  designNodeDoubleClick?: (nodeId: string) => void;
};

type FlowEdgeData = {
  edgeType: 'success' | 'always';
  mapping?: Record<string, string>;
};

type Point = { x: number; y: number };
type SelectableFlowElement = { selected?: boolean };

function isSelected(element: SelectableFlowElement): boolean {
  return !!element.selected;
}

export function useComposerCanvas(options: UseComposerCanvasOptions) {
  const {
    onConnect,
    onEdgeClick,
    onNodeClick,
    onNodeDoubleClick,
    onNodeDragStop,
    onPaneClick,
    fitView,
    getEdges,
    getNodes,
    setEdges,
    setNodes,
  } = useVueFlow();

  const nodes = ref<Node[]>([]);
  const edges = ref<Edge[]>([]);
  const fittedPipelineId = ref<string | null>(null);
  const nodeTypes: Record<string, any> = { pipelineNode: markRaw(PipelineNodeCard) };
  const defaultEdgeOptions = {
    type: 'smoothstep',
    animated: false,
    pathOptions: { offset: 20, borderRadius: 12 },
  };
  const { layout } = useLayout();

  const selectedNodes = computed<Node[]>(() => {
    const currentNodes = nodes.value as Node[];
    const selected: Node[] = [];
    for (const node of currentNodes) {
      if (isSelected(node as SelectableFlowElement)) selected.push(node);
    }
    return selected;
  });
  const selectedEdges = computed<Edge[]>(() => {
    const currentEdges = edges.value as Edge[];
    const selected: Edge[] = [];
    for (const edge of currentEdges) {
      if (isSelected(edge as SelectableFlowElement)) selected.push(edge);
    }
    return selected;
  });
  const hasSelection = computed(
    () => selectedNodes.value.length + selectedEdges.value.length > 0,
  );
  const showEmptyState = computed(
    () => options.designMode() && nodes.value.length === 0,
  );

  function edgeMappings(data?: FlowEdgeData): Array<[string, string]> {
    return Object.entries(data?.mapping ?? {}).filter(
      (entry): entry is [string, string] =>
        typeof entry[0] === 'string' && typeof entry[1] === 'string',
    );
  }

  function edgeTooltip(data?: FlowEdgeData): string {
    const mappings = edgeMappings(data);
    return mappings.length > 0
      ? mappings.map(([targetField, sourceField]) => `${sourceField} → ${targetField}`).join('\n')
      : 'No mapping configured';
  }

  function edgeIndicator(edge: EdgeProps<FlowEdgeData>): 'mapped' | 'warning' | null {
    if (edgeMappings(edge.data).length > 0) return 'mapped';
    return (edge.targetNode.data as FlowNodeData | undefined)?.type === 'tool'
      ? 'warning'
      : null;
  }

  function edgePath(edge: EdgeProps<FlowEdgeData>): { d: string; x: number; y: number } {
    const [d, x, y] = getSmoothStepPath({ ...edge, offset: 20, borderRadius: 12 });
    return { d, x, y };
  }

  function syncCanvas(definition: PipelineDefinition, view?: PipelineView | null): void {
    const cached = loadPositions(view?.pipelineId);
    const hasCachedPositions =
      definition.nodes.length > 0 && definition.nodes.every((node) => cached[node.id]);

    let nextNodes: Node[] = pipelineNodesToFlow(definition.nodes, view?.pipelineId, view?.nodes);
    const nextEdges: Edge[] = pipelineEdgesToFlow(definition.edges);

    if (!hasCachedPositions && nextNodes.length > 0) {
      nextNodes = layout(nextNodes, nextEdges, 'LR');
    }

    nodes.value = nextNodes;
    edges.value = nextEdges;
    setNodes(nextNodes);
    setEdges(nextEdges);
  }

  function emitDefinitionUpdate(): void {
    const nextDefinition = definitionFromFlow(getNodes.value, getEdges.value);
    options.updateDefinition(nextDefinition);
  }

  function persistPositions(nextNodes: Node[]): void {
    savePositions(nextNodes, options.view()?.pipelineId);
  }

  function syncLocalGraph(nextNodes: Node[], nextEdges: Edge[]): void {
    nodes.value = nextNodes;
    edges.value = nextEdges;
    setNodes(nextNodes);
    setEdges(nextEdges);
    persistPositions(nextNodes);
    emitDefinitionUpdate();
  }

  const { clearPendingDefinitionSync, scheduleDefinitionSync, shouldSyncCanvasImmediately } =
    useComposerSync(syncCanvas, options.designMode);

  watch(
    options.localDefinition,
    (definition, previousDefinition) => {
      if (!definition || !options.designMode()) return;
      if (shouldSyncCanvasImmediately(definition, previousDefinition)) {
        clearPendingDefinitionSync();
        syncCanvas(definition, options.view());
        nextTick(() => fitView());
        return;
      }

      scheduleDefinitionSync(definition, options.view());
    },
    { immediate: true },
  );

  watch(
    options.designMode,
    (designMode) => {
      if (designMode) {
        fittedPipelineId.value = null;
      }
    },
    { immediate: true },
  );

  watch(
    () => options.view()?.pipelineId ?? null,
    (pipelineId, previousPipelineId) => {
      if (pipelineId !== previousPipelineId) {
        fittedPipelineId.value = null;
      }
    },
  );

  watch(
    options.view,
    (view) => {
      if (!view || options.designMode()) return;
      syncCanvas(
        {
          nodes: view.nodes.map(toPipelineNodeFromViewNode),
          edges: view.edges,
        },
        view,
      );
      const loadedPipelineId = view.pipelineId ?? null;
      if (loadedPipelineId && loadedPipelineId !== fittedPipelineId.value) {
        fittedPipelineId.value = loadedPipelineId;
        nextTick(() => fitView({ padding: 0.15, duration: 200 }));
      }
    },
    { deep: true, immediate: true },
  );

  onNodeDragStop(() => {
    savePositions(getNodes.value, options.view()?.pipelineId);
    emitDefinitionUpdate();
  });

  onConnect((connection: Connection) => {
    if (!options.designMode() || !connection.source || !connection.target) return;

    const currentEdges = edges.value as Edge[];
    const nextEdges: Edge[] = [
      ...currentEdges,
      {
        id: `${connection.source}->${connection.target}`,
        source: connection.source,
        target: connection.target,
        type: 'smoothstep',
        label: 'success',
        data: { edgeType: 'success' },
      },
    ];
    edges.value = nextEdges;
    setEdges(nextEdges);
    emitDefinitionUpdate();
  });

  onNodeClick(({ node }) => {
    options.selectNode(node.id);
    options.selectEdge(null);
  });

  onEdgeClick(({ edge }) => {
    options.selectNode(null);
    options.selectEdge({ source: edge.source, target: edge.target });
  });

  onPaneClick(() => {
    options.selectNode(null);
    options.selectEdge(null);
  });

  const zoomStore = useSemanticZoomStore();
  const sessionStore = useSessionStore();
  const appShellStore = useAppShellStore();

  function enterInspect(nodeId: string, event: MouseEvent | TouchEvent): void {
    const target = event.target;
    if (!(target instanceof Element)) return;
    const nodeEl = target.closest<HTMLElement>('.vue-flow__node');
    if (!nodeEl) return;
    zoomStore.enterInspect(nodeId, nodeEl.getBoundingClientRect());
  }

  async function handleNodeDoubleClick(node: Node, event: MouseEvent | TouchEvent): Promise<void> {
    if (options.designMode()) {
      options.designNodeDoubleClick?.(node.id);
      return;
    }

    const viewNode = options.view()?.nodes.find((candidate) => candidate.nodeId === node.id);
    const canReconnect = viewNode?.provenance?.detail.canReconnect
      ?? (viewNode?.type !== 'tool' && !!viewNode?.acpSessionId);
    const acpSessionId = viewNode?.provenance?.detail.sessionId
      ?? (viewNode?.type !== 'tool' ? viewNode?.acpSessionId : undefined);

    if (!viewNode || !canReconnect || !acpSessionId) {
      enterInspect(node.id, event);
      return;
    }

    try {
      const result = await sessionStore.focusOrResumeRemoteSession(acpSessionId);
      if (result.status === 'activated' || result.status === 'resumed') {
        appShellStore.openAgents();
      }
    } catch (error) {
      sessionStore.setError(
        error instanceof Error
          ? `Failed to reconnect ACP session: ${error.message}`
          : 'Failed to reconnect ACP session.',
      );
    }
  }

  onNodeDoubleClick(({ node, event }) => {
    void handleNodeDoubleClick(node, event);
  });

  function handleFlowKeyDown(event: KeyboardEvent): void {
    if (event.key !== 'Enter' && event.key !== ' ') return;

    const nodeEl = (event.target as Element).closest<HTMLElement>('.vue-flow__node');
    if (!nodeEl?.dataset.id) return;

    event.preventDefault();
    options.selectNode(nodeEl.dataset.id);
    options.selectEdge(null);
  }

  function deleteSelected(): void {
    if (!options.designMode()) return;

    const nodeIds = new Set(selectedNodes.value.map((node) => node.id));
    const edgeIds = new Set(selectedEdges.value.map((edge) => edge.id));

    const nextNodes = getNodes.value.filter((node) => !nodeIds.has(node.id));
    const nextEdges = getEdges.value.filter(
      (edge) => !edgeIds.has(edge.id) && !nodeIds.has(edge.source) && !nodeIds.has(edge.target),
    );
    syncLocalGraph(nextNodes, nextEdges);
  }

  function handleAutoLayout(direction: 'LR' | 'TB'): void {
    const currentNodes: Node[] = getNodes.value;
    const currentEdges: Edge[] = getEdges.value;
    const nextNodes: Node[] = layout(currentNodes, currentEdges, direction);
    nodes.value = nextNodes;
    setNodes(nextNodes);
    persistPositions(nextNodes);
    nextTick(() => fitView());
  }

  function cloneExecutionFields(
    node: Extract<PipelineNode, { type: 'prompt' | 'sequentialThinking' | 'tool' }>,
  ) {
    return {
      ...(node.model ? { model: node.model } : {}),
      ...(node.thoughtLevel ? { thoughtLevel: node.thoughtLevel } : {}),
      ...(node.allowedTools ? { allowedTools: [...node.allowedTools] } : {}),
      ...(typeof node.maxTurns === 'number' ? { maxTurns: node.maxTurns } : {}),
      ...(typeof node.timeout === 'number' ? { timeout: node.timeout } : {}),
      ...(node.agentConfig
        ? {
            agentConfig: {
              ...node.agentConfig,
              ...(node.agentConfig.configOptions
                ? { configOptions: { ...node.agentConfig.configOptions } }
                : {}),
            },
          }
        : {}),
    };
  }

  function cloneNodeWithNewId(node: PipelineNode): PipelineNode {
    const type = node.type ?? 'workflow';
    const id = createNodeId(type);

    if (isWorkflowNode(node)) {
      return createNode('workflow', {
        id,
        ...(node.displayLabel ? { displayLabel: node.displayLabel } : {}),
        workflowId: node.workflowId,
        agent: node.acpClient ?? '',
        ...(node.prompt ? { prompt: node.prompt } : {}),
      });
    }

    if (isPromptNode(node)) {
      return createNode('prompt', {
        id,
        ...(node.displayLabel ? { displayLabel: node.displayLabel } : {}),
        prompt: node.prompt,
        agent: node.agent,
        ...(node.toolMode ? { toolMode: node.toolMode } : {}),
        ...cloneExecutionFields(node),
      });
    }

    if (isSequentialThinkingNode(node)) {
      return createNode('sequentialThinking', {
        id,
        ...(node.displayLabel ? { displayLabel: node.displayLabel } : {}),
        topic: node.topic,
        agent: node.agent,
        ...(typeof node.totalThoughts === 'number' ? { totalThoughts: node.totalThoughts } : {}),
        ...(node.analysisType ? { analysisType: node.analysisType } : {}),
        ...cloneExecutionFields(node),
      });
    }

    if (isToolNode(node)) {
      return createNode('tool', {
        id,
        ...(node.displayLabel ? { displayLabel: node.displayLabel } : {}),
        toolName: node.toolName,
        ...(node.agent ? { agent: node.agent } : {}),
        ...(node.toolArgs ? { toolArgs: { ...node.toolArgs } } : {}),
        ...cloneExecutionFields(node),
      });
    }

    const unreachable: never = node;
    throw new Error(`Unsupported node type: ${JSON.stringify(unreachable)}`);
  }

  function findDefinitionNode(nodeId: string): PipelineNode | undefined {
    return options.localDefinition()?.nodes.find((node) => node.id === nodeId);
  }

  function toFlowPosition(clientX: number, clientY: number, rootElement: HTMLElement): Point {
    const rootRect = rootElement.getBoundingClientRect();
    const viewportElement = rootElement.querySelector<HTMLElement>('.vue-flow__viewport');
    const transform = viewportElement ? getComputedStyle(viewportElement).transform : 'none';
    const matrix = transform && transform !== 'none'
      ? new DOMMatrixReadOnly(transform)
      : new DOMMatrixReadOnly();
    const zoom = matrix.a || 1;

    return {
      x: Math.round(((clientX - rootRect.left - matrix.m41) / zoom) / 20) * 20,
      y: Math.round(((clientY - rootRect.top - matrix.m42) / zoom) / 20) * 20,
    };
  }

  function createNodeAt(node: PipelineNode, position: Point): void {
    if (!options.designMode()) return;

    const currentNodes = getNodes.value as Node[];
    const nextNodes = [
      ...currentNodes,
      {
        id: node.id,
        type: 'pipelineNode',
        position,
        data: { ...pipelineNodesToFlow([node])[0]!.data },
      } satisfies Node,
    ];
    syncLocalGraph(nextNodes, getEdges.value);
    options.selectNode(node.id);
    options.selectEdge(null);
  }

  function deleteNode(nodeId: string): void {
    if (!options.designMode()) return;

    const nextNodes = getNodes.value.filter((node) => node.id !== nodeId);
    const nextEdges = getEdges.value.filter(
      (edge) => edge.source !== nodeId && edge.target !== nodeId,
    );
    syncLocalGraph(nextNodes, nextEdges);
    options.selectNode(null);
    options.selectEdge(null);
  }

  function disconnectNodeEdges(nodeId: string): void {
    if (!options.designMode()) return;

    const nextEdges = getEdges.value.filter(
      (edge) => edge.source !== nodeId && edge.target !== nodeId,
    );
    syncLocalGraph(getNodes.value, nextEdges);
    options.selectNode(nodeId);
    options.selectEdge(null);
  }

  function duplicateNode(nodeId: string, offset = 50): void {
    if (!options.designMode()) return;

    const sourceNode = findDefinitionNode(nodeId);
    const sourceFlowNode = getNodes.value.find((node) => node.id === nodeId);
    if (!sourceNode || !sourceFlowNode) return;

    createNodeAt(cloneNodeWithNewId(sourceNode), {
      x: sourceFlowNode.position.x + offset,
      y: sourceFlowNode.position.y + offset,
    });
  }

  function editNode(nodeId: string): void {
    options.selectNode(nodeId);
    options.selectEdge(null);
  }

  return {
    createNodeAt,
    deleteNode,
    defaultEdgeOptions,
    deleteSelected,
    disconnectNodeEdges,
    edgeIndicator,
    edgePath,
    edgeTooltip,
    editNode,
    edges,
    fitView,
    handleAutoLayout,
    handleFlowKeyDown,
    hasSelection,
    nodeTypes,
    nodes,
    showEmptyState,
    toFlowPosition,
    duplicateNode,
    zoomStore,
  };
}
