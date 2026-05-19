// Pipeline edge and definition types.

export type EdgeType = "success" | "always" | "conditional";

export interface EdgeBinding {
  from: string;
  to: string;
}

export type EdgeMode = "context" | "binding" | "none";

export type EdgeConditionEquals = string | number | boolean;

export interface EdgeCondition {
  path: string;
  equals: EdgeConditionEquals;
}

import type { PipelineNode } from "./pipeline-types-node";

export interface PipelineEdge {
  source: string;
  target: string;
  type?: EdgeType;
  /**
   * Backend wire format for conditional edges.
   * Flat fields are expected by pipeline MCP APIs.
   */
  path?: string;
  equals?: EdgeConditionEquals;
  /**
   * Local UI convenience shape used by the canvas/editor.
   */
  condition?: EdgeCondition;
  mode?: EdgeMode;
  bindings?: EdgeBinding[];
  mapping?: Record<string, string>;
}

export type PipelineInputs = Record<string, unknown>;

export interface PipelineDefinition {
  nodes: PipelineNode[];
  edges: PipelineEdge[];
  inputs?: PipelineInputs;
  inputSchema?: Record<string, unknown>;
  /**
   * Optional template identifier used to re-derive default human-readable
   * labels for pipelines instantiated from a template after backend
   * round-trips and app reloads.
   */
  templateId?: string;
}

export type NodeStatus =
  | "pending"
  | "ready"
  | "active"
  | "completed"
  | "abandoned"
  | "stuck"
  | "retrying";

export type PipelineStatus =
  | "active"
  | "running"
  | "paused"
  | "cancelled"
  | "completed"
  | "abandoned"
  | "stuck";
