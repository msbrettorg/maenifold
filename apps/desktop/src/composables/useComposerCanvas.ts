import { computed, markRaw, ref } from 'vue';
import type { Ref } from 'vue';
import { useVueFlow } from '@vue-flow/core';
import type { Connection, Edge, Node } from '@vue-flow/core';

import PipelineNodeCard from '../components/PipelineNodeCard.vue';
import { useComposerSync } from './useComposerSync';
import { useLayout } from './useLayout';
import {
  definitionFromFlow,
  loadPositions,
  pipelineEdgesToFlow,
  pipelineNodesToFlow,
  savePositions,
} from '../lib/pipeline-composer-flow';
import type { PipelineDefinition } from '../lib/pipeline-types-edge';
import type { PipelineView } from '../lib/pipeline-types-view';
import {
  edgeIndicator,
  edgeLabel,
  edgeLabelTitle,
  edgePath,
  edgePathStyle,
  edgeTooltip,
} from '../lib/pipeline-composer-edge-utils';
import { createComposerNodeActions } from '../lib/pipeline-composer-node-actions';
import {
  applyDirectionHandles,
  DEFAULT_DIRECTION,
  DIRECTION_HANDLES,
  isSelected,
  toFlowPosition,
  type Direction,
  type SelectableFlowElement,
} from '../lib/composer-canvas-helpers';
import { useSemanticZoomStore } from '../stores/semantic-zoom';
import { createComposerCanvasHandlers, type ComposerCanvasHandlersOptions } from './useComposerCanvasHandlers';
import { setupComposerCanvasWatchers } from './useComposerCanvasWatchers';

type SelectedEdge = { source: string; target: string };

type UseComposerCanvasOptions = {
  designMode: () => boolean;
  localDefinition: () => PipelineDefinition | undefined;
  view: () => PipelineView | null | undefined;
  layoutDirection?: Ref<'LR' | 'TB'>;
  updateDefinition: (definition: PipelineDefinition) => void;
  selectNode: (nodeId: string | null) => void;
  selectEdge: (edge: SelectedEdge | null) => void;
  designNodeDoubleClick?: (nodeId: string) => void;
  edgeDoubleClick?: (edge: SelectedEdge) => void;
};

export function useComposerCanvas(options: UseComposerCanvasOptions) {
  const layoutDirection = options.layoutDirection ?? ref<Direction>(DEFAULT_DIRECTION);
  const {
    onConnect,
    onEdgeClick,
    onEdgeDoubleClick,
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

  async function syncCanvas(
    definition: PipelineDefinition,
    view?: PipelineView | null,
    layoutOptions?: { forceLayout?: boolean; skipLayout?: boolean },
  ): Promise<void> {
    const cached = loadPositions(view?.pipelineId);

    let nextNodes: Node[] = pipelineNodesToFlow(definition.nodes, view?.pipelineId, view?.nodes);
    let nextEdges: Edge[] = pipelineEdgesToFlow(definition.edges);

    const fixedPositions = Object.fromEntries(
      nextNodes
        .filter((node) => cached[node.id])
        .map((node) => [node.id, cached[node.id]]),
    ) as Record<string, { x: number; y: number }>;
    const hasUncachedNodes = nextNodes.some((node) => !fixedPositions[node.id]);
    const shouldLayout =
      nextNodes.length > 0 &&
      !layoutOptions?.skipLayout &&
      (layoutOptions?.forceLayout || hasUncachedNodes || Object.keys(fixedPositions).length === 0);

    if (shouldLayout) {
      const result = await layout(
        nextNodes,
        applyDirectionHandles(nextEdges, layoutDirection.value),
        layoutDirection.value,
        { fixedPositions },
      );
      nextNodes = result.nodes;
    }

    nextEdges = applyDirectionHandles(nextEdges, layoutDirection.value);

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
    const directedEdges = applyDirectionHandles(nextEdges, layoutDirection.value);
    nodes.value = nextNodes;
    edges.value = directedEdges;
    setNodes(nextNodes);
    setEdges(directedEdges);
    persistPositions(nextNodes);
    emitDefinitionUpdate();
  }

  const { clearPendingDefinitionSync, scheduleDefinitionSync, shouldSyncCanvasImmediately } =
    useComposerSync(syncCanvas, options.designMode);

  setupComposerCanvasWatchers({
    designMode: options.designMode,
    localDefinition: options.localDefinition,
    view: options.view,
    layoutDirection,
    fittedPipelineId,
    fitView,
    syncCanvas,
    shouldSyncCanvasImmediately,
    clearPendingDefinitionSync,
    scheduleDefinitionSync,
  });

  onNodeDragStop(({ node }) => {
    void node;
    savePositions(getNodes.value, options.view()?.pipelineId);
    emitDefinitionUpdate();
  });

  onConnect((connection: Connection) => {
    if (!options.designMode() || !connection.source || !connection.target) return;
    const handles = DIRECTION_HANDLES[layoutDirection.value] ?? DIRECTION_HANDLES.LR;

    const currentEdges = edges.value as Edge[];
    const nextEdges: Edge[] = [
      ...currentEdges,
      {
        id: `${connection.source}->${connection.target}`,
        source: connection.source,
        target: connection.target,
        sourceHandle: connection.sourceHandle ?? handles.source,
        targetHandle: connection.targetHandle ?? handles.target,
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

  const handlerOptions: ComposerCanvasHandlersOptions = {
    designMode: options.designMode,
    selectNode: options.selectNode,
    selectEdge: options.selectEdge,
    designNodeDoubleClick: options.designNodeDoubleClick,
    layoutDirection,
    selectedNodes,
    selectedEdges,
    nodes: nodes as unknown as { value: Node[] },
    edges: edges as unknown as { value: Edge[] },
    getNodes: () => getNodes.value,
    getEdges: () => getEdges.value,
    setNodes: setNodes as (nodes: Node[]) => unknown,
    setEdges: setEdges as (edges: Edge[]) => unknown,
    fitView,
    persistPositions,
    syncLocalGraph,
    zoomStore,
  };
  const handlers = createComposerCanvasHandlers(handlerOptions);

  onNodeDoubleClick(({ node, event }) => {
    void handlers.handleNodeDoubleClick(node, event);
  });

  onEdgeDoubleClick(({ edge }) => {
    options.edgeDoubleClick?.({ source: edge.source, target: edge.target });
  });

  const nodeActions = createComposerNodeActions({
    designMode: options.designMode,
    localDefinition: options.localDefinition,
    selectNode: options.selectNode,
    selectEdge: options.selectEdge,
    getNodes: () => getNodes.value,
    getEdges: () => getEdges.value,
    syncLocalGraph,
  });

  return {
    createNodeAt: nodeActions.createNodeAt,
    deleteNode: nodeActions.deleteNode,
    defaultEdgeOptions,
    deleteSelected: handlers.deleteSelected,
    disconnectNodeEdges: nodeActions.disconnectNodeEdges,
    edgeIndicator,
    edgeLabel,
    edgeLabelTitle,
    edgePath,
    edgePathStyle,
    edgeTooltip,
    editNode: nodeActions.editNode,
    edges,
    fitView,
    handleAutoLayout: handlers.handleAutoLayout,
    handleFlowKeyDown: handlers.handleFlowKeyDown,
    hasSelection,
    nodeTypes,
    nodes,
    showEmptyState,
    toFlowPosition,
    duplicateNode: nodeActions.duplicateNode,
    zoomStore,
  };
}
