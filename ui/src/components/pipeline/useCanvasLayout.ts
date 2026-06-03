// ELK `layered` async layout for the Pipelines canvas.
//
// ELK is graph-layout-as-a-promise: `elk.layout(graph)` resolves with computed
// child coordinates. The canvas awaits this BEFORE handing nodes to Vue Flow so
// there is never a flash of unpositioned nodes. We use the bundled ELK build
// (`elk.bundled.js`) which runs in-thread — no worker-URL bundling step — which
// is well within budget for the pipeline graphs we render (tens of nodes).
//
// Incremental layout contract: callers pass the set of already-fixed positions
// (the per-pipeline localStorage cache). Those nodes are pinned via
// `elk.position` and, on the consumer side, their cached coordinates are always
// preferred over ELK's output — so grafting/new nodes get laid out while every
// existing hand-placed node keeps its exact position.

import ELK from 'elkjs/lib/elk.bundled.js';
import type { ElkExtendedEdge, ElkNode } from 'elkjs/lib/elk-api';
import type { Edge, Node } from '@vue-flow/core';
import { NODE_H, NODE_W, type Direction, type XY } from './canvasBridge';

const elk = new ELK();

const ELK_DIRECTION: Record<Direction, string> = {
  LR: 'RIGHT',
  TB: 'DOWN',
};

export function useCanvasLayout() {
  /**
   * Compute positions for `nodes`. Returns a map of nodeId → {x,y}. On any ELK
   * failure it returns an empty map so the caller can fall back to cached or
   * grid positions without throwing.
   */
  async function layout(
    nodes: Node[],
    edges: Edge[],
    direction: Direction,
    fixedPositions: Record<string, XY> = {}
  ): Promise<Record<string, XY>> {
    if (nodes.length === 0) return {};

    const children: ElkNode[] = nodes.map((node) => {
      const fixed = fixedPositions[node.id];
      const layoutOptions: Record<string, string> = {};
      if (fixed) {
        layoutOptions['elk.position'] = `(${Math.round(fixed.x)}, ${Math.round(fixed.y)})`;
      }
      return {
        id: node.id,
        width: NODE_W,
        height: NODE_H,
        ...(fixed ? { x: fixed.x, y: fixed.y } : {}),
        layoutOptions,
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
        'elk.direction': ELK_DIRECTION[direction] ?? 'RIGHT',
        'elk.layered.spacing.nodeNodeBetweenLayers': '120',
        'elk.spacing.nodeNode': '64',
        'elk.layered.considerModelOrder.strategy': 'NODES_AND_EDGES',
      },
      children,
      edges: elkEdges,
    };

    try {
      const result = await elk.layout(graph);
      const positions: Record<string, XY> = {};
      for (const child of result.children ?? []) {
        positions[child.id] = { x: child.x ?? 0, y: child.y ?? 0 };
      }
      return positions;
    } catch (error) {
      console.error('[pipeline-canvas] ELK layout failed', {
        direction,
        nodeCount: nodes.length,
        edgeCount: edges.length,
        error,
      });
      return {};
    }
  }

  return { layout };
}
