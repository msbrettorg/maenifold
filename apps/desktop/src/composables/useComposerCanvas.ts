import { computed, markRaw, nextTick, ref, watch } from 'vue';
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
  toPipelineNodeFromViewNode,
} from '../lib/pipeline-composer-flow';
import type { PipelineDefinition, PipelineView } from '../lib/pipeline-types';
import { edgeIndicator, edgePath, edgeTooltip } from '../lib/pipeline-composer-edge-utils';
import { createComposerNodeActions } from '../lib/pipeline-composer-node-actions';
import { useSemanticZoomStore } from '../stores/semantic-zoom';

type SelectedEdge = { source: string; target: string };

type UseComposerCanvasOptions = {
  designMode: () => boolean;
  localDefinition: () => PipelineDefinition | undefined;
  view: () => PipelineView | null | undefined;
  updateDefinition: (definition: PipelineDefinition) => void;
  selectNode: (nodeId: string | null) => void;
  selectEdge: (edge: SelectedEdge | null) => void;
  designNodeDoubleClick?: (nodeId: string) => void;
  edgeDoubleClick?: (edge: SelectedEdge) => void;
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

  /** Apply ELK edge routing points to flow edges after auto-layout. */
  function applyElkEdgePaths(
    edgeList: Edge[],
    edgePaths: Map<string, Array<{ x: number; y: number }>>,
  ): Edge[] {
    return edgeList.map((edge) => {
      const points = edgePaths.get(edge.id);
      if (!points || points.length < 2) return edge;
      return { ...edge, data: { ...edge.data, elkPoints: points } };
    });
  }

  /** Strip ELK edge routing points so edges revert to smoothstep calculation. */
  function clearElkEdgePaths(edgeList: Edge[]): Edge[] {
    return edgeList.map((edge) => {
      if (!edge.data?.elkPoints) return edge;
      const { elkPoints: _, ...rest } = edge.data;
      return { ...edge, data: rest };
    });
  }

  async function syncCanvas(definition: PipelineDefinition, view?: PipelineView | null): Promise<void> {
    const cached = loadPositions(view?.pipelineId);
    const hasCachedPositions =
      definition.nodes.length > 0 && definition.nodes.every((node) => cached[node.id]);

    let nextNodes: Node[] = pipelineNodesToFlow(definition.nodes, view?.pipelineId, view?.nodes);
    let nextEdges: Edge[] = pipelineEdgesToFlow(definition.edges);

    if (!hasCachedPositions && nextNodes.length > 0) {
      const result = await layout(nextNodes, nextEdges, 'TB');
      nextNodes = result.nodes;
      nextEdges = applyElkEdgePaths(nextEdges, result.edgePaths);
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
    async (definition, previousDefinition) => {
      if (!definition || !options.designMode()) return;
      if (shouldSyncCanvasImmediately(definition, previousDefinition)) {
        clearPendingDefinitionSync();
        await syncCanvas(definition, options.view());
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
    async (view) => {
      if (!view || options.designMode()) return;
      await syncCanvas(
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
    // ELK edge paths become stale after manual node repositioning — clear them
    // so edges fall back to the smoothstep path calculation.
    const currentEdges = getEdges.value;
    if (currentEdges.some((e) => e.data?.elkPoints)) {
      const clearedEdges = clearElkEdgePaths(currentEdges);
      edges.value = clearedEdges;
      setEdges(clearedEdges);
    }
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
    enterInspect(node.id, event);
  }

  onNodeDoubleClick(({ node, event }) => {
    void handleNodeDoubleClick(node, event);
  });

  onEdgeDoubleClick(({ edge }) => {
    options.edgeDoubleClick?.({ source: edge.source, target: edge.target });
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

  async function handleAutoLayout(direction: 'LR' | 'TB'): Promise<void> {
    const currentNodes: Node[] = getNodes.value;
    const currentEdges: Edge[] = getEdges.value;
    const result = await layout(currentNodes, currentEdges, direction);
    const nextEdges = applyElkEdgePaths(currentEdges, result.edgePaths);
    nodes.value = result.nodes;
    edges.value = nextEdges;
    setNodes(result.nodes);
    setEdges(nextEdges);
    persistPositions(result.nodes);
    nextTick(() => fitView());
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
    deleteSelected,
    disconnectNodeEdges: nodeActions.disconnectNodeEdges,
    edgeIndicator,
    edgePath,
    edgeTooltip,
    editNode: nodeActions.editNode,
    edges,
    fitView,
    handleAutoLayout,
    handleFlowKeyDown,
    hasSelection,
    nodeTypes,
    nodes,
    showEmptyState,
    toFlowPosition,
    duplicateNode: nodeActions.duplicateNode,
    zoomStore,
  };
}
