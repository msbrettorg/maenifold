import dagre from '@dagrejs/dagre';
import type { Edge, Node } from '@vue-flow/core';

export function useLayout() {
  function layout(
    nodes: Node[],
    edges: Edge[],
    direction: 'LR' | 'TB' = 'LR',
  ): Node[] {
    const graph = new dagre.graphlib.Graph();
    const nodeWidth = 280;
    const nodeHeight = 220;

    graph.setDefaultEdgeLabel(() => ({}));
    graph.setGraph({ rankdir: direction, ranksep: 200, nodesep: 80 });

    for (const node of nodes) graph.setNode(node.id, { width: nodeWidth, height: nodeHeight });
    for (const edge of edges) graph.setEdge(edge.source, edge.target);

    dagre.layout(graph);

    return nodes.map((node) => {
      const { x, y } = graph.node(node.id);
      const pos = { x: x - nodeWidth / 2, y: y - nodeHeight / 2 };
      return { ...node, position: pos, computedPosition: { ...pos, z: 0 } };
    });
  }

  return { layout };
}
