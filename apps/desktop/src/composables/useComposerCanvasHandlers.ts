// Composer canvas event handlers + selection-driven actions. Extracted from
// composables/useComposerCanvas.ts to keep the main composable under 300 LOC.
import { nextTick } from 'vue';
import type { ComputedRef, Ref } from 'vue';
import type { Edge, Node } from '@vue-flow/core';
import {
  applyDirectionHandles,
  type Direction,
} from '../lib/composer-canvas-helpers';
import { useLayout } from './useLayout';

type SelectedEdge = { source: string; target: string };

interface ZoomStoreLike {
  enterInspect: (nodeId: string, rect: DOMRect) => void;
}

export interface ComposerCanvasHandlersOptions {
  designMode: () => boolean;
  selectNode: (nodeId: string | null) => void;
  selectEdge: (edge: SelectedEdge | null) => void;
  designNodeDoubleClick?: (nodeId: string) => void;
  layoutDirection: Ref<Direction>;
  selectedNodes: ComputedRef<Node[]>;
  selectedEdges: ComputedRef<Edge[]>;
  nodes: { value: Node[] };
  edges: { value: Edge[] };
  getNodes: () => Node[];
  getEdges: () => Edge[];
  setNodes: (nodes: Node[]) => unknown;
  setEdges: (edges: Edge[]) => unknown;
  fitView: (options?: { padding?: number; duration?: number }) => void;
  persistPositions: (nodes: Node[]) => void;
  syncLocalGraph: (nextNodes: Node[], nextEdges: Edge[]) => void;
  zoomStore: ZoomStoreLike;
}

export interface ComposerCanvasHandlers {
  enterInspect: (nodeId: string, event: MouseEvent | TouchEvent) => void;
  handleNodeDoubleClick: (node: Node, event: MouseEvent | TouchEvent) => Promise<void>;
  handleFlowKeyDown: (event: KeyboardEvent) => void;
  deleteSelected: () => void;
  handleAutoLayout: (direction: Direction) => Promise<void>;
}

export function createComposerCanvasHandlers(
  options: ComposerCanvasHandlersOptions,
): ComposerCanvasHandlers {
  const { layout } = useLayout();

  function enterInspect(nodeId: string, event: MouseEvent | TouchEvent): void {
    const target = event.target;
    if (!(target instanceof Element)) return;
    const nodeEl = target.closest<HTMLElement>('.vue-flow__node');
    if (!nodeEl) return;
    options.zoomStore.enterInspect(nodeId, nodeEl.getBoundingClientRect());
  }

  async function handleNodeDoubleClick(
    node: Node,
    event: MouseEvent | TouchEvent,
  ): Promise<void> {
    if (options.designMode()) {
      options.designNodeDoubleClick?.(node.id);
      return;
    }
    enterInspect(node.id, event);
  }

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

    const nodeIds = new Set(options.selectedNodes.value.map((node) => node.id));
    const edgeIds = new Set(options.selectedEdges.value.map((edge) => edge.id));

    const nextNodes = options.getNodes().filter((node) => !nodeIds.has(node.id));
    const nextEdges = options.getEdges().filter(
      (edge) => !edgeIds.has(edge.id) && !nodeIds.has(edge.source) && !nodeIds.has(edge.target),
    );
    options.syncLocalGraph(nextNodes, nextEdges);
  }

  async function handleAutoLayout(direction: Direction): Promise<void> {
    options.layoutDirection.value = direction;
    const currentNodes: Node[] = options.getNodes();
    const currentEdges: Edge[] = options.getEdges();
    const result = await layout(
      currentNodes,
      applyDirectionHandles(currentEdges, direction),
      direction,
    );
    const nextEdges = applyDirectionHandles(currentEdges, direction);
    options.nodes.value = result.nodes;
    options.edges.value = nextEdges;
    options.setNodes(result.nodes);
    options.setEdges(nextEdges);
    options.persistPositions(result.nodes);
    nextTick(() => options.fitView());
  }

  return {
    enterInspect,
    handleNodeDoubleClick,
    handleFlowKeyDown,
    deleteSelected,
    handleAutoLayout,
  };
}
