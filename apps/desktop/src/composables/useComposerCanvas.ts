import { computed, markRaw, nextTick, ref, watch } from 'vue';
import { useVueFlow } from '@vue-flow/core';
import type { Connection, Edge, Node } from '@vue-flow/core';

import PipelineNodeCard from '../components/PipelineNodeCard.vue';
import { useComposerSync } from './useComposerSync';
import { useLayout } from './useLayout';
import { usePipelineNodeSessionLauncher } from './usePipelineNodeSessionLauncher';
import {
  definitionFromFlow,
  loadPositions,
  pipelineEdgesToFlow,
  pipelineNodesToFlow,
  savePositions,
  toPipelineNodeFromViewNode,
} from '../lib/pipeline-composer-flow';
import { createComposerNodeActions } from '../lib/pipeline-composer-node-actions';
import {
  edgeIndicator,
  edgePath,
  edgeTooltip,
} from '../lib/pipeline-composer-edge-utils';
import type { PipelineDefinition, PipelineView } from '../lib/pipeline-types';
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

  const selectedNodes = computed<Node[]>(() =>
    (nodes.value as Node[]).filter((node) => isSelected(node as SelectableFlowElement)),
  );
  const selectedEdges = computed<Edge[]>(() =>
    (edges.value as Edge[]).filter((edge) => isSelected(edge as SelectableFlowElement)),
  );
  const hasSelection = computed(() => selectedNodes.value.length + selectedEdges.value.length > 0);
  const showEmptyState = computed(() => options.designMode() && nodes.value.length === 0);

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

  watch(options.designMode, (designMode) => {
    if (designMode) fittedPipelineId.value = null;
  }, { immediate: true });
  watch(
    () => options.view()?.pipelineId ?? null,
    (pipelineId, previousPipelineId) => {
      if (pipelineId !== previousPipelineId) fittedPipelineId.value = null;
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
    const nextEdges: Edge[] = [...(edges.value as Edge[]), {
      id: `${connection.source}->${connection.target}`,
      source: connection.source,
      target: connection.target,
      type: 'smoothstep',
      label: 'success',
      data: { edgeType: 'success' },
    }];
    edges.value = nextEdges;
    setEdges(nextEdges);
    emitDefinitionUpdate();
  });
  onNodeClick(({ node }) => { options.selectNode(node.id); options.selectEdge(null); });
  onEdgeClick(({ edge }) => { options.selectNode(null); options.selectEdge({ source: edge.source, target: edge.target }); });
  onPaneClick(() => { options.selectNode(null); options.selectEdge(null); });

  const zoomStore = useSemanticZoomStore();

  function enterInspect(nodeId: string, event: MouseEvent | TouchEvent): void {
    const target = event.target;
    if (!(target instanceof Element)) return;
    const nodeEl = target.closest<HTMLElement>('.vue-flow__node');
    if (!nodeEl) return;
    zoomStore.enterInspect(nodeId, nodeEl.getBoundingClientRect());
  }

  const { handleNodeDoubleClick } = usePipelineNodeSessionLauncher(options.view, enterInspect);

  onNodeDoubleClick(({ node, event }) => {
    if (options.designMode()) {
      options.designNodeDoubleClick?.(node.id);
      return;
    }

    void handleNodeDoubleClick(node.id, event);
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

  function handleAutoLayout(direction: 'LR' | 'TB'): void {
    const currentNodes: Node[] = getNodes.value;
    const currentEdges: Edge[] = getEdges.value;
    const nextNodes: Node[] = layout(currentNodes, currentEdges, direction);
    nodes.value = nextNodes;
    setNodes(nextNodes);
    persistPositions(nextNodes);
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

  const { createNodeAt, deleteNode, disconnectNodeEdges, duplicateNode, editNode } =
    createComposerNodeActions({
      designMode: options.designMode,
      localDefinition: options.localDefinition,
      selectNode: options.selectNode,
      selectEdge: options.selectEdge,
      getNodes: () => getNodes.value,
      getEdges: () => getEdges.value,
      syncLocalGraph,
    });

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
