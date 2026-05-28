import { useEffect, useMemo } from "react";
import "@xyflow/react/dist/style.css";
import {
  Background,
  BackgroundVariant,
  Controls,
  MiniMap,
  ReactFlow,
  type Edge,
  type Node,
} from "@xyflow/react";

import {
  getNodeTone,
  type PipelineNode,
  type PipelineView,
} from "@/lib/pipeline";

import { DagNode } from "./DagNode";

type DagProps = {
  pipeline: PipelineView;
  selectedNodeId: string | null;
  onSelectNode: (nodeId: string) => void;
};

type DagData = {
  node: PipelineNode;
  tone: ReturnType<typeof getNodeTone>;
};

const nodeTypes = {
  pipeline: DagNode,
};

export function Dag({ pipeline, selectedNodeId, onSelectNode }: DagProps) {
  const laidOutNodes = useMemo(() => layoutNodes(pipeline.nodes, pipeline.edges), [pipeline.edges, pipeline.nodes]);

  const nodes = useMemo<Node<DagData>[]>(
    () =>
      pipeline.nodes.map((node) => ({
        id: node.nodeId,
        type: "pipeline",
        position: laidOutNodes.get(node.nodeId) ?? { x: 0, y: 0 },
        selected: node.nodeId === selectedNodeId,
        data: {
          node,
          tone: getNodeTone(node.status),
        },
      })),
    [laidOutNodes, pipeline.nodes, selectedNodeId],
  );

  const edges = useMemo<Edge[]>(
    () =>
      pipeline.edges.map((edge, index) => ({
        id: `${edge.source}-${edge.target}-${index}`,
        source: edge.source,
        target: edge.target,
        type: "bezier",
        animated: pipeline.active.includes(edge.source),
      })),
    [pipeline.active, pipeline.edges],
  );

  useEffect(() => {
    if (!selectedNodeId && pipeline.nodes[0]) {
      onSelectNode(pipeline.nodes[0].nodeId);
    }
  }, [onSelectNode, pipeline.nodes, selectedNodeId]);

  return (
    <div className="h-full rounded-xl border border-border bg-card/60">
      <ReactFlow
        key={pipeline.pipelineId}
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        fitView
        proOptions={{ hideAttribution: true }}
        nodesDraggable={false}
        nodesConnectable={false}
        elementsSelectable
        onNodeClick={(_, node) => onSelectNode(node.id)}
        className="pipeline-flow"
      >
        <Background variant={BackgroundVariant.Dots} gap={16} size={1} />
        <MiniMap
          pannable
          zoomable
          className="!rounded-lg !border !border-border !bg-card"
          nodeColor={(node) => toneColor(node.data?.tone)}
        />
        <Controls
          showInteractive={false}
          className="pipeline-controls"
        />
      </ReactFlow>
    </div>
  );
}

function toneColor(tone: ReturnType<typeof getNodeTone> | undefined): string {
  switch (tone) {
    case "ready":
      return "var(--color-chart-5)";
    case "active":
      return "var(--color-primary)";
    case "done":
      return "var(--color-chart-2)";
    case "failed":
      return "var(--color-destructive)";
    default:
      return "var(--color-border)";
  }
}

function layoutNodes(nodes: PipelineNode[], edges: PipelineView["edges"]) {
  const order = new Map(nodes.map((node, index) => [node.nodeId, index]));
  const incoming = new Map<string, string[]>();
  const outgoing = new Map<string, string[]>();
  const indegree = new Map<string, number>();
  const depth = new Map<string, number>();

  for (const node of nodes) {
    incoming.set(node.nodeId, []);
    outgoing.set(node.nodeId, []);
    indegree.set(node.nodeId, 0);
    depth.set(node.nodeId, 0);
  }

  for (const edge of edges) {
    incoming.get(edge.target)?.push(edge.source);
    outgoing.get(edge.source)?.push(edge.target);
    indegree.set(edge.target, (indegree.get(edge.target) ?? 0) + 1);
  }

  const queue = nodes
    .filter((node) => (indegree.get(node.nodeId) ?? 0) === 0)
    .map((node) => node.nodeId)
    .sort((left, right) => (order.get(left) ?? 0) - (order.get(right) ?? 0));

  while (queue.length > 0) {
    const current = queue.shift();
    if (!current) {
      break;
    }

    for (const next of outgoing.get(current) ?? []) {
      depth.set(next, Math.max(depth.get(next) ?? 0, (depth.get(current) ?? 0) + 1));
      indegree.set(next, (indegree.get(next) ?? 1) - 1);
      if ((indegree.get(next) ?? 0) === 0) {
        queue.push(next);
        queue.sort((left, right) => (order.get(left) ?? 0) - (order.get(right) ?? 0));
      }
    }
  }

  const columns = new Map<number, string[]>();

  for (const node of nodes) {
    const column = depth.get(node.nodeId) ?? 0;
    if (!columns.has(column)) {
      columns.set(column, []);
    }
    columns.get(column)?.push(node.nodeId);
  }

  const positions = new Map<string, { x: number; y: number }>();
  const columnWidth = 320;
  const rowHeight = 150;

  for (const [column, columnNodes] of columns.entries()) {
    columnNodes.forEach((nodeId, row) => {
      positions.set(nodeId, {
        x: column * columnWidth,
        y: row * rowHeight,
      });
    });
  }

  return positions;
}
