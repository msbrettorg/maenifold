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
import type { PipelineDefinition, PipelineView } from '../lib/pipeline-types';
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
};

type FlowEdgeData = {
  edgeType: 'success' | 'always';
  mapping?: Record<string, string>;
};

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
  const nodeTypes: Record<string, any> = { pipelineNode: markRaw(PipelineNodeCard) };
  const defaultEdgeOptions = {
    type: 'smoothstep',
    animated: false,
    pathOptions: { offset: 20, borderRadius: 12 },
  };
  const { layout } = useLayout();

  const selectedNodes = computed(() =>
    nodes.value.filter((node) => (node as Node & { selected?: boolean }).selected),
  );
  const selectedEdges = computed(() =>
    edges.value.filter((edge) => (edge as Edge & { selected?: boolean }).selected),
  );
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

    nodes.value = pipelineNodesToFlow(definition.nodes, view?.pipelineId, view?.nodes);
    edges.value = pipelineEdgesToFlow(definition.edges);

    if (!hasCachedPositions && nodes.value.length > 0) {
      nodes.value = layout(nodes.value, edges.value, 'LR');
    }

    setNodes(nodes.value);
    setEdges(edges.value);
  }

  function emitDefinitionUpdate(): void {
    options.updateDefinition(definitionFromFlow(getNodes.value, getEdges.value));
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
    options.view,
    (view) => {
      if (!view || options.designMode()) return;
      syncCanvas(
        { nodes: view.nodes.map(toPipelineNodeFromViewNode), edges: view.edges },
        view,
      );
      nextTick(() => fitView());
    },
    { deep: true, immediate: true },
  );

  onNodeDragStop(() => {
    savePositions(getNodes.value, options.view()?.pipelineId);
    emitDefinitionUpdate();
  });

  onConnect((connection: Connection) => {
    if (!options.designMode() || !connection.source || !connection.target) return;

    edges.value = [
      ...edges.value,
      {
        id: `${connection.source}->${connection.target}`,
        source: connection.source,
        target: connection.target,
        type: 'smoothstep',
        label: 'success',
        data: { edgeType: 'success' },
      },
    ];
    setEdges(edges.value);
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
    if (options.designMode()) return;

    const viewNode = options.view()?.nodes.find((candidate) => candidate.nodeId === node.id);
    const acpSessionId = viewNode?.type !== 'tool' ? viewNode?.acpSessionId : undefined;

    if (!viewNode || !acpSessionId) {
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

    nodes.value = getNodes.value.filter((node) => !nodeIds.has(node.id));
    edges.value = getEdges.value.filter(
      (edge) => !edgeIds.has(edge.id) && !nodeIds.has(edge.source) && !nodeIds.has(edge.target),
    );

    setNodes(nodes.value);
    setEdges(edges.value);
    emitDefinitionUpdate();
  }

  function handleAutoLayout(direction: 'LR' | 'TB'): void {
    nodes.value = layout(getNodes.value, getEdges.value, direction);
    setNodes(nodes.value);
    nextTick(() => fitView());
  }

  return {
    defaultEdgeOptions,
    deleteSelected,
    edgeIndicator,
    edgePath,
    edgeTooltip,
    edges,
    fitView,
    handleAutoLayout,
    handleFlowKeyDown,
    hasSelection,
    nodeTypes,
    nodes,
    showEmptyState,
    zoomStore,
  };
}
