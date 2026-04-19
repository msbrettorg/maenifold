import ELK from 'elkjs/lib/elk-api';
import type { ElkNode, ElkExtendedEdge } from 'elkjs/lib/elk-api';
import type { Edge, Node } from '@vue-flow/core';
import elkWorkerUrl from 'elkjs/lib/elk-worker.min.js?url';
import { toast } from 'vue-sonner';

const elk = new ELK({ workerUrl: elkWorkerUrl });

const NODE_WIDTH = 280;
const NODE_HEIGHT = 220;

const ELK_DIRECTION: Record<string, string> = {
  LR: 'RIGHT',
  TB: 'DOWN',
};

type LayoutOptions = {
  fixedPositions?: Record<string, { x: number; y: number }>;
};

function fallbackGridPosition(index: number): { x: number; y: number } {
  return { x: (index % 4) * 340 + 40, y: Math.floor(index / 4) * 240 + 40 };
}

export type LayoutResult = {
  nodes: Node[];
};

export function useLayout() {
  async function layout(
    nodes: Node[],
    edges: Edge[],
    direction: 'LR' | 'TB' = 'LR',
    options?: LayoutOptions,
  ): Promise<LayoutResult> {
    if (nodes.length === 0) return { nodes: [] };
    const fixedPositions = options?.fixedPositions ?? {};

    const elkNodes: ElkNode[] = nodes.map((node) => {
      const fixed = fixedPositions[node.id];
      const nodeLayoutOptions: Record<string, string> = {};
      if (fixed) {
        nodeLayoutOptions['elk.position'] = `(${Math.round(fixed.x)}, ${Math.round(fixed.y)})`;
      }

      return {
        id: node.id,
        width: NODE_WIDTH,
        height: NODE_HEIGHT,
        ...(fixed ? { x: fixed.x, y: fixed.y } : {}),
        layoutOptions: nodeLayoutOptions,
      };
    });

    const elkEdges: ElkExtendedEdge[] = edges.map((edge) => ({
      id: edge.id,
      sources: [edge.source],
      targets: [edge.target],
    }));

    const graph: ElkNode = {
      id: 'root',
      layoutOptions: {
        'elk.algorithm': 'layered',
        'elk.direction': ELK_DIRECTION[direction] ?? 'DOWN',
        'elk.layered.spacing.nodeNodeBetweenLayers': '220',
        'elk.spacing.nodeNode': '96',
      },
      children: elkNodes,
      edges: elkEdges,
    };

    try {
      const layoutResult = await elk.layout(graph);

      const positionMap = new Map<string, { x: number; y: number }>();
      for (const child of layoutResult.children ?? []) {
        positionMap.set(child.id, { x: child.x ?? 0, y: child.y ?? 0 });
      }

      const layoutNodes = nodes.map((node, index) => {
        const fallback = fixedPositions[node.id] ?? node.position ?? fallbackGridPosition(index);
        const pos = positionMap.get(node.id) ?? fallback;
        return { ...node, position: pos, computedPosition: { ...pos, z: 0 } };
      });

      return { nodes: layoutNodes };
    } catch (error) {
      console.error('[pipeline-composer] ELK layout failed', {
        direction,
        nodeCount: nodes.length,
        edgeCount: edges.length,
        error,
      });
      toast.warning('Auto-layout failed. Using fallback layout.');

      const fallbackNodes = nodes.map((node, index) => {
        const pos = fixedPositions[node.id] ?? node.position ?? fallbackGridPosition(index);
        return { ...node, position: pos, computedPosition: { ...pos, z: 0 } };
      });

      return { nodes: fallbackNodes };
    }
  }

  return { layout };
}
