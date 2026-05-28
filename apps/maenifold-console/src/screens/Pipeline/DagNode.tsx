import { Handle, Position, type NodeProps } from "@xyflow/react";

import { Badge } from "@/components/ui/badge";
import {
  deriveNodeDuration,
  getNodeLabel,
  type PipelineNode,
  type PipelineNodeTone,
} from "@/lib/pipeline";

type DagNodeData = {
  node: PipelineNode;
  tone: PipelineNodeTone;
};

const TONE_STYLES: Record<PipelineNodeTone, string> = {
  pending: "border-border bg-card text-card-foreground",
  ready: "border-chart-5/40 bg-chart-5/10 text-foreground",
  active: "border-primary/40 bg-primary/10 text-foreground shadow-[0_0_0_1px_var(--color-primary)]",
  done: "border-chart-2/40 bg-chart-2/10 text-foreground",
  failed: "border-destructive/40 bg-destructive/10 text-foreground",
};

const BADGE_STYLES: Record<PipelineNodeTone, string> = {
  pending: "border-border/70 bg-muted/50 text-muted-foreground",
  ready: "border-chart-5/30 bg-chart-5/10 text-chart-5",
  active: "border-primary/30 bg-primary/15 text-primary",
  done: "border-chart-2/30 bg-chart-2/10 text-chart-2",
  failed: "border-destructive/30 bg-destructive/10 text-destructive",
};

export function DagNode({ data, selected }: NodeProps<DagNodeData>) {
  return (
    <div
      className={[
        "min-w-[220px] rounded-xl border px-4 py-3 transition-all",
        TONE_STYLES[data.tone],
        selected ? "ring-2 ring-ring/60" : "",
      ].join(" ")}
    >
      <Handle type="target" position={Position.Left} className="!size-3 !border-2 !border-background !bg-border" />
      <div className="mb-2 flex items-center justify-between gap-3">
        <Badge variant="outline" className={BADGE_STYLES[data.tone]}>
          {data.node.status.toUpperCase()}
        </Badge>
        <span className="font-mono text-[11px] uppercase tracking-[0.16em] text-muted-foreground">
          {data.node.type}
        </span>
      </div>
      <div className="text-sm font-medium leading-tight">{getNodeLabel(data.node)}</div>
      <div className="mt-2 flex items-center justify-between gap-2 font-mono text-xs text-muted-foreground">
        <span>{data.node.sessionId}</span>
        <span>{deriveNodeDuration(data.node)}</span>
      </div>
      <Handle type="source" position={Position.Right} className="!size-3 !border-2 !border-background !bg-border" />
    </div>
  );
}
