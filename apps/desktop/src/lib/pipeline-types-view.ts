// Pipeline view (server-side runtime) types.

import type {
  PipelineNodeAdopt,
  PipelineNodeAgentConfig,
  SessionRef,
} from "./pipeline-types-node";
import type {
  NodeStatus,
  PipelineEdge,
  PipelineInputs,
  PipelineStatus,
} from "./pipeline-types-edge";

interface ViewNodeBase {
  nodeId: string;
  displayLabel?: string;
  sessionId: string;
  acpSessionId?: string;
  thinkingSessionId?: string;
  provenance?: NodeProvenance;
  status: NodeStatus;
  attempts: number;
  output?: string;
  failureReason?: string | null;
}

export interface NodeProvenanceDetail {
  sessionId?: string | null;
  thinkingSessionId?: string | null;
  parentWorkflowId?: string | null;
  lineageGateTaskId?: string | null;
  lineagePipelineRunId?: string | null;
  lineageThinkingSessionId?: string | null;
  lineageMemoryNoteId?: string | null;
  parent_workflow_id?: string | null;
  lineage_gate_task_id?: string | null;
  lineage_pipeline_run_id?: string | null;
  lineage_thinking_session_id?: string | null;
  lineage_memory_note_id?: string | null;
  adoption?: {
    role: string;
    color: string;
    perspective: string;
  };
}

export interface NodeProvenance {
  agent?: string | null;
  inspectable: boolean;
  detail: NodeProvenanceDetail;
}

export interface ToolProvenance {
  toolName: string;
  resolvedArgs?: Record<string, unknown> | null;
  responseSummary?: string | null;
}

export interface WorkflowStepEntry {
  step_name: string;
  status: string;
  output?: string | null;
}

export interface WorkflowViewNode extends ViewNodeBase {
  type?: "workflow";
  workflowId: string;
  acpClient?: string;
  adopt?: PipelineNodeAdopt;
  requiresRole?: string;
  prompt?: string;
  cwd?: string;
  model?: string;
  thoughtLevel?: string;
  agentConfig?: PipelineNodeAgentConfig;
  timeout?: number;
  assembled_prompt?: string | null;
  workflow_steps?: WorkflowStepEntry[] | null;
}

export interface PromptViewNode extends ViewNodeBase {
  type: "prompt";
  prompt: string;
  agent: string;
  adopt?: PipelineNodeAdopt;
  requiresRole?: string;
  toolMode?: string;
  model?: string;
  thoughtLevel?: string;
  allowedTools?: string[];
  maxTurns?: number;
  cwd?: string;
  timeout?: number;
  agentConfig?: PipelineNodeAgentConfig;
  sessionRef?: SessionRef;
  assembled_prompt?: string | null;
}

export interface SequentialThinkingViewNode extends ViewNodeBase {
  type: "sequentialThinking";
  topic: string;
  agent: string;
  adopt?: PipelineNodeAdopt;
  requiresRole?: string;
  totalThoughts?: number;
  analysisType?: string;
  model?: string;
  thoughtLevel?: string;
  allowedTools?: string[];
  maxTurns?: number;
  cwd?: string;
  timeout?: number;
  agentConfig?: PipelineNodeAgentConfig;
  assembled_prompt?: string | null;
  thinking_trace?: Array<{
    thought_number: number;
    total_thoughts?: number;
    text: string;
    is_revision?: boolean;
    revises_thought?: number | null;
    branch_id?: string | null;
    agent?: string | null;
  }> | null;
}

export interface ToolViewNode extends ViewNodeBase {
  type: "tool";
  toolName: string;
  toolArgs?: Record<string, unknown>;
  toolProvenance?: ToolProvenance | null;
  agent?: string;
  model?: string;
  thoughtLevel?: string;
  allowedTools?: string[];
  maxTurns?: number;
  cwd?: string;
  timeout?: number;
  agentConfig?: PipelineNodeAgentConfig;
}

export type ViewNode =
  | WorkflowViewNode
  | PromptViewNode
  | SequentialThinkingViewNode
  | ToolViewNode;

export interface PipelineView {
  pipelineId: string;
  status: PipelineStatus;
  templateId?: string;
  inputs?: PipelineInputs;
  nodes: ViewNode[];
  edges: PipelineEdge[];
  ready: string[];
  active: string[];
  completed: string[];
  mermaid: string;
  running: boolean;
}

export interface PipelineRunSummary {
  timestamp?: string | number | null;
  startedAt?: string | number | null;
  completedAt?: string | number | null;
  updatedAt?: string | number | null;
  createdAt?: string | number | null;
}

export interface PipelineListItem {
  pipelineId: string;
  status: PipelineStatus;
  nodeCount: number;
  completedCount: number;
  description?: string | null;
  lastRunAt?: string | number | null;
  updatedAt?: string | number | null;
  modifiedAt?: string | number | null;
  runs?: PipelineRunSummary[] | null;
}

export interface LogEntry {
  nodeId: string;
  from: string;
  to: string;
  timestamp: number;
  preview?: string;
}

export function getViewNodeDisplayLabel(node: ViewNode): string {
  return node.displayLabel ?? node.nodeId;
}
