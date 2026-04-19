import { computed, markRaw, nextTick, ref, watch } from 'vue';
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
  toPipelineNodeFromViewNode,
} from '../lib/pipeline-composer-flow';
import type { PipelineDefinition, PipelineView } from '../lib/pipeline-types';
import { edgeIndicator, edgeLabel, edgeLabelTitle, edgePath, edgePathStyle, edgeTooltip } from '../lib/pipeline-composer-edge-utils';
import { createComposerNodeActions } from '../lib/pipeline-composer-node-actions';
import { useSemanticZoomStore } from '../stores/semantic-zoom';

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

type Point = { x: number; y: number };
type SelectableFlowElement = { selected?: boolean };

type Direction = 'LR' | 'TB';

const DIRECTION_HANDLES: Record<Direction, { source: string; target: string }> = {
  LR: { source: 'out-right', target: 'in-left' },
  TB: { source: 'out-bottom', target: 'in-top' },
};

function isSelected(element: SelectableFlowElement): boolean {
  return !!element.selected;
}

export function useComposerCanvas(options: UseComposerCanvasOptions) {
  const layoutDirection = options.layoutDirection ?? ref<Direction>('LR');
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

  /**
   * Annotate every edge with its position among siblings sharing the same source
   * node. The smoothstep fallback uses these to spread fan-outs into distinct
   * lanes so ≥4 outgoing edges from one source no longer overlap on a single
   * horizontal/vertical bus.
   */
  function annotateEdgeSiblings(edgeList: Edge[]): Edge[] {
    const groups = new Map<string, string[]>();
    for (const edge of edgeList) {
      const ids = groups.get(edge.source) ?? [];
      ids.push(edge.id);
      groups.set(edge.source, ids);
    }
    return edgeList.map((edge) => {
      const sibs = groups.get(edge.source) ?? [edge.id];
      const siblingTotal = sibs.length;
      const siblingIndex = sibs.indexOf(edge.id);
      return {
        ...edge,
        data: { ...edge.data, siblingIndex, siblingTotal },
      };
    });
  }

  function clearElkEdgePathsForNode(edgeList: Edge[], nodeId: string): Edge[] {
    return edgeList.map((edge) => {
      if (edge.source !== nodeId && edge.target !== nodeId) return edge;
      if (!edge.data?.elkPoints) return edge;
      const { elkPoints: _, ...rest } = edge.data;
      return { ...edge, data: rest };
    });
  }

  function applyDirectionHandles(edgeList: Edge[], direction: Direction): Edge[] {
    const handles = DIRECTION_HANDLES[direction] ?? DIRECTION_HANDLES.LR;
    return edgeList.map((edge) => ({
      ...edge,
      sourceHandle: handles.source,
      targetHandle: handles.target,
    }));
  }

  function mergeExistingElkPaths(nextEdges: Edge[], currentEdges: Edge[]): Edge[] {
    const existingPaths = new Map<string, Array<{ x: number; y: number }>>();
    for (const edge of currentEdges) {
      const points = edge.data?.elkPoints;
      if (points && points.length >= 2) existingPaths.set(edge.id, points);
    }

    return nextEdges.map((edge) => {
      const points = existingPaths.get(edge.id);
      if (!points) return edge;
      return { ...edge, data: { ...edge.data, elkPoints: points } };
    });
  }

  function hasViewStructureChanged(
    view: PipelineView | null | undefined,
    previousView: PipelineView | null | undefined,
  ): boolean {
    if (!view && !previousView) return false;
    if (!view || !previousView) return true;

    const nodeIds = new Set(view.nodes.map((node) => node.nodeId));
    const previousNodeIds = new Set(previousView.nodes.map((node) => node.nodeId));
    if (nodeIds.size !== previousNodeIds.size) return true;
    for (const nodeId of nodeIds) {
      if (!previousNodeIds.has(nodeId)) return true;
    }

    const edgePairs = new Set(view.edges.map((edge) => `${edge.source}->${edge.target}`));
    const previousEdgePairs = new Set(
      previousView.edges.map((edge) => `${edge.source}->${edge.target}`),
    );
    if (edgePairs.size !== previousEdgePairs.size) return true;
    for (const pair of edgePairs) {
      if (!previousEdgePairs.has(pair)) return true;
    }

    return false;
  }

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
      nextEdges = applyElkEdgePaths(nextEdges, result.edgePaths);
    } else {
      nextEdges = mergeExistingElkPaths(nextEdges, getEdges.value);
    }

    nextEdges = annotateEdgeSiblings(applyDirectionHandles(nextEdges, layoutDirection.value));

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
    const annotatedEdges = annotateEdgeSiblings(
      applyDirectionHandles(nextEdges, layoutDirection.value),
    );
    nodes.value = nextNodes;
    edges.value = annotatedEdges;
    setNodes(nextNodes);
    setEdges(annotatedEdges);
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
    async (view, previousView) => {
      if (!view || options.designMode()) return;
      const structureChanged = hasViewStructureChanged(view, previousView);
      await syncCanvas(
        {
          nodes: view.nodes.map(toPipelineNodeFromViewNode),
          edges: view.edges,
        },
        view,
        {
          forceLayout: structureChanged,
          skipLayout: !structureChanged,
        },
      );
      const loadedPipelineId = view.pipelineId ?? null;
      if (loadedPipelineId && loadedPipelineId !== fittedPipelineId.value) {
        fittedPipelineId.value = loadedPipelineId;
        nextTick(() => fitView({ padding: 0.15, duration: 200 }));
      }
    },
    { immediate: true },
  );

  onNodeDragStop(({ node }) => {
    // ELK edge paths become stale after manual node repositioning — clear them
    // so edges fall back to the smoothstep path calculation.
    const currentEdges = getEdges.value;
    const draggedNodeId = node?.id;
    if (draggedNodeId && currentEdges.some((e) => e.data?.elkPoints)) {
      const clearedEdges = annotateEdgeSiblings(
        clearElkEdgePathsForNode(currentEdges, draggedNodeId),
      );
      edges.value = clearedEdges;
      setEdges(clearedEdges);
    }
    savePositions(getNodes.value, options.view()?.pipelineId);
    emitDefinitionUpdate();
  });

  onConnect((connection: Connection) => {
    if (!options.designMode() || !connection.source || !connection.target) return;
    const handles = DIRECTION_HANDLES[layoutDirection.value] ?? DIRECTION_HANDLES.LR;

    const currentEdges = edges.value as Edge[];
    const nextEdges: Edge[] = annotateEdgeSiblings([
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
    ]);
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
    layoutDirection.value = direction;
    const currentNodes: Node[] = getNodes.value;
    const currentEdges: Edge[] = getEdges.value;
    const result = await layout(
      currentNodes,
      applyDirectionHandles(currentEdges, direction),
      direction,
    );
    const nextEdges = annotateEdgeSiblings(
      applyDirectionHandles(
        applyElkEdgePaths(currentEdges, result.edgePaths),
        direction,
      ),
    );
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
    edgeLabel,
    edgeLabelTitle,
    edgePath,
    edgePathStyle,
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
