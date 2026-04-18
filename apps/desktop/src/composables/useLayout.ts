import ELK from 'elkjs/lib/elk-api';
import type { ElkNode, ElkExtendedEdge } from 'elkjs/lib/elk-api';
import type { Edge, Node } from '@vue-flow/core';
import elkWorkerUrl from 'elkjs/lib/elk-worker.min.js?url';

const elk = new ELK({ workerUrl: elkWorkerUrl });

const NODE_WIDTH = 280;
const NODE_HEIGHT = 220;

const ELK_DIRECTION: Record<string, string> = {
  LR: 'RIGHT',
  TB: 'DOWN',
};

export type LayoutResult = {
  nodes: Node[];
  edgePaths: Map<string, Array<{ x: number; y: number }>>;
};

export function useLayout() {
  async function layout(
    nodes: Node[],
    edges: Edge[],
    direction: 'LR' | 'TB' = 'TB',
  ): Promise<LayoutResult> {
    if (nodes.length === 0) return { nodes: [], edgePaths: new Map() };

    const elkNodes: ElkNode[] = nodes.map((node) => ({
      id: node.id,
      width: NODE_WIDTH,
      height: NODE_HEIGHT,
    }));

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
        // Pull edges away from neighbouring nodes so fan-outs from a single
        // source don't collapse onto the node's own border.
        'elk.spacing.edgeNode': '48',
        // Force parallel edges to occupy distinct lanes instead of stacking on
        // the same horizontal bus.
        'elk.spacing.edgeEdge': '32',
        'elk.layered.spacing.edgeNodeBetweenLayers': '64',
        'elk.layered.spacing.edgeEdgeBetweenLayers': '32',
        // Run interactive crossing minimisation so ELK preserves a deterministic
        // sibling ordering that maps back to source-handle index.
        'elk.layered.crossingMinimization.semiInteractive': 'true',
        'elk.edgeRouting': 'ORTHOGONAL',
      },
      children: elkNodes,
      edges: elkEdges,
    };

    const layoutResult = await elk.layout(graph);

    const positionMap = new Map<string, { x: number; y: number }>();
    for (const child of layoutResult.children ?? []) {
      positionMap.set(child.id, { x: child.x ?? 0, y: child.y ?? 0 });
    }

    const edgePaths = new Map<string, Array<{ x: number; y: number }>>();
    for (const edge of layoutResult.edges ?? []) {
      const section = edge.sections?.[0];
      if (!section) continue;
      const points: Array<{ x: number; y: number }> = [
        section.startPoint,
        ...(section.bendPoints ?? []),
        section.endPoint,
      ];
      if (points.length >= 2) {
        edgePaths.set(edge.id, points);
      }
    }

    const layoutNodes = nodes.map((node) => {
      const pos = positionMap.get(node.id) ?? { x: 0, y: 0 };
      return { ...node, position: pos, computedPosition: { ...pos, z: 0 } };
    });

    return { nodes: layoutNodes, edgePaths };
  }

  return { layout };
}
