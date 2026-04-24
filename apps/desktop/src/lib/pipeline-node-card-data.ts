import type {
  NodeStatus,
  PipelineNode,
  PipelineNodeAdopt,
  PipelineStatus,
} from './pipeline-types';

export type NodeCardData = {
  nodeId: string;
  displayLabel?: string;
  model?: string;
  workflowId?: string;
  acpClient?: string;
  adopt?: PipelineNodeAdopt;
  requiresRole?: string;
  status?: NodeStatus | PipelineStatus | 'running';
  attempts?: number;
  type?: PipelineNode['type'];
  prompt?: string;
  agent?: string;
  topic?: string;
  totalThoughts?: number;
  thinkingTraceLength?: number;
  analysisType?: string;
  toolName?: string;
  toolArgs?: Record<string, unknown>;
  failureReason?: string | null;
};

const PROMPT_PREFIX = 'prompt: ';
const THINKING_PREFIX = 'think: ';

export function shortenText(text: string, max = 48): string {
  return text.length > max ? `${text.slice(0, max).trimEnd()}...` : text;
}

function stripPrefix(value: string | undefined, prefix: string): string {
  return value?.startsWith(prefix) ? value.slice(prefix.length) : (value ?? '');
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

export function inferPipelineNodeFromCardData(data: NodeCardData): PipelineNode {
  const preview = stripPrefix(data.workflowId, PROMPT_PREFIX);
  const topic = stripPrefix(data.workflowId, THINKING_PREFIX);

  if (data.type === 'workflow' || (data.workflowId && !data.type)) {
    return {
      id: data.nodeId,
      type: 'workflow',
      workflowId: data.workflowId ?? data.nodeId,
      ...(data.acpClient ? { acpClient: data.acpClient } : {}),
      ...(data.adopt ? { adopt: data.adopt } : {}),
      ...(data.requiresRole ? { requiresRole: data.requiresRole } : {}),
      ...(data.prompt ? { prompt: data.prompt } : {}),
    };
  }

  if (data.type === 'prompt' || data.workflowId?.startsWith(PROMPT_PREFIX)) {
    return {
      id: data.nodeId,
      type: 'prompt',
      agent: data.agent ?? data.acpClient ?? 'agent',
      prompt: (data.prompt ?? preview) || data.displayLabel || data.nodeId,
      ...(data.adopt ? { adopt: data.adopt } : {}),
      ...(data.requiresRole ? { requiresRole: data.requiresRole } : {}),
    };
  }

  if (data.type === 'sequentialThinking' || data.topic || data.workflowId?.startsWith(THINKING_PREFIX)) {
    return {
      id: data.nodeId,
      type: 'sequentialThinking',
      topic: (data.topic ?? topic) || data.displayLabel || data.nodeId,
      agent: data.agent ?? data.acpClient ?? 'agent',
      ...(data.adopt ? { adopt: data.adopt } : {}),
      ...(data.requiresRole ? { requiresRole: data.requiresRole } : {}),
      ...(typeof data.totalThoughts === 'number' ? { totalThoughts: data.totalThoughts } : {}),
      ...(data.analysisType ? { analysisType: data.analysisType } : {}),
    };
  }

  if (data.type === 'tool' || data.toolName) {
    return {
      id: data.nodeId,
      type: 'tool',
      toolName: data.toolName ?? data.displayLabel ?? data.nodeId,
      ...(data.agent ? { agent: data.agent } : {}),
      ...(isRecord(data.toolArgs) ? { toolArgs: data.toolArgs } : {}),
    };
  }

  return {
    id: data.nodeId,
    type: 'workflow',
    workflowId: data.workflowId ?? data.displayLabel ?? data.nodeId,
    ...(data.acpClient ? { acpClient: data.acpClient } : {}),
    ...(data.prompt ? { prompt: data.prompt } : {}),
  };
}
