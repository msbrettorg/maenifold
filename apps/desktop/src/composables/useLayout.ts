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

const ACTIVE_PORTS = {
  LR: { input: 'in-left', output: 'out-right', inputSide: 'WEST', outputSide: 'EAST' },
  TB: { input: 'in-top', output: 'out-bottom', inputSide: 'NORTH', outputSide: 'SOUTH' },
} as const;

type LayoutOptions = {
  fixedPositions?: Record<string, { x: number; y: number }>;
};

function fallbackGridPosition(index: number): { x: number; y: number } {
  return { x: (index % 4) * 340 + 40, y: Math.floor(index / 4) * 240 + 40 };
}

export type LayoutResult = {
  nodes: Node[];
  edgePaths: Map<string, Array<{ x: number; y: number }>>;
};

export function useLayout() {
  async function layout(
    nodes: Node[],
    edges: Edge[],
    direction: 'LR' | 'TB' = 'LR',
    options?: LayoutOptions,
  ): Promise<LayoutResult> {
    if (nodes.length === 0) return { nodes: [], edgePaths: new Map() };
    const fixedPositions = options?.fixedPositions ?? {};
    const activePorts = ACTIVE_PORTS[direction] ?? ACTIVE_PORTS.LR;

    // ELK requires globally unique port IDs across the entire graph. Vue Flow
    // handle IDs (e.g. 'in-left') are scoped per-node in the DOM, but ELK's
    // port registry is flat — reusing the same id on every node causes ELK to
    // either fail validation or fall back to implicit attachment points that
    // don't align with the declared WEST/EAST/NORTH/SOUTH sides. We therefore
    // namespace the ELK port id with the node id, and edges must reference
    // those qualified ids (not the bare node id) so the routed sections
    // actually start/end at the declared port positions.
    const elkPortId = (nodeId: string, localPortId: string): string =>
      `${nodeId}::${localPortId}`;

    // Vue Flow renders handles at fixed DOM positions (centered on each side
    // via `top: 50%` / `left: 50%`). For the routed edge endpoints to land on
    // top of the visible connector dots we must pin the ELK port positions to
    // the same centers — FIXED_SIDE alone lets ELK shift ports along the side
    // for routing optimization, which produces visually disconnected lines.
    const portCenterForSide = (side: string): { x: number; y: number } => {
      switch (side) {
        case 'WEST':
          return { x: 0, y: NODE_HEIGHT / 2 };
        case 'EAST':
          return { x: NODE_WIDTH, y: NODE_HEIGHT / 2 };
        case 'NORTH':
          return { x: NODE_WIDTH / 2, y: 0 };
        case 'SOUTH':
        default:
          return { x: NODE_WIDTH / 2, y: NODE_HEIGHT };
      }
    };
    const inputPortPos = portCenterForSide(activePorts.inputSide);
    const outputPortPos = portCenterForSide(activePorts.outputSide);

    const elkNodes: ElkNode[] = nodes.map((node) => {
      const fixed = fixedPositions[node.id];
      const nodeLayoutOptions: Record<string, string> = {
        'elk.portConstraints': 'FIXED_POS',
      };
      if (fixed) {
        nodeLayoutOptions['elk.position'] = `(${Math.round(fixed.x)}, ${Math.round(fixed.y)})`;
      }

      return {
        id: node.id,
        width: NODE_WIDTH,
        height: NODE_HEIGHT,
        ...(fixed ? { x: fixed.x, y: fixed.y } : {}),
        layoutOptions: nodeLayoutOptions,
        ports: [
          {
            id: elkPortId(node.id, activePorts.input),
            x: inputPortPos.x,
            y: inputPortPos.y,
            width: 0,
            height: 0,
            layoutOptions: { 'port.side': activePorts.inputSide },
          },
          {
            id: elkPortId(node.id, activePorts.output),
            x: outputPortPos.x,
            y: outputPortPos.y,
            width: 0,
            height: 0,
            layoutOptions: { 'port.side': activePorts.outputSide },
          },
        ],
      };
    });

    const elkEdges: ElkExtendedEdge[] = edges.map((edge) => ({
      id: edge.id,
      sources: [elkPortId(edge.source, activePorts.output)],
      targets: [elkPortId(edge.target, activePorts.input)],
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

    try {
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

      const layoutNodes = nodes.map((node, index) => {
        const fallback = fixedPositions[node.id] ?? node.position ?? fallbackGridPosition(index);
        const pos = positionMap.get(node.id) ?? fallback;
        return { ...node, position: pos, computedPosition: { ...pos, z: 0 } };
      });

      return { nodes: layoutNodes, edgePaths };
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

      return { nodes: fallbackNodes, edgePaths: new Map() };
    }
  }

  return { layout };
}
