// Design-mode pure operations: node constructors for the add-menu, a unique-id
// generator, and the pre-save validity gate. Kept framework-free so the rules
// are unit-testable without a DOM or a store.

import type {
  PipelineDefinition,
  PipelineEdge,
  PipelineNode,
  PromptNode,
  SequentialThinkingNode,
  ToolNode,
  WorkflowNode,
} from './pipeline-types';

// The add-menu choices map to a node constructor. `add-session-node` is a Prompt
// node that additionally carries the required `sessionRef` (agent + sessionId).
export type AddNodeKind =
  | 'ask-agent'
  | 'add-session-node'
  | 'run-tool'
  | 'think'
  | 'run-workflow';

export const ADD_NODE_LABELS: Record<AddNodeKind, string> = {
  'ask-agent': 'Ask agent (prompt)',
  'add-session-node': 'Add session node (prompt)',
  'run-tool': 'Run tool',
  think: 'Think (sequential)',
  'run-workflow': 'Run workflow',
};

/** Generate an id not already used by any node in `existing`. */
export function uniqueNodeId(existing: readonly PipelineNode[]): string {
  const used = new Set(existing.map((node) => node.id));
  // crypto.randomUUID is available in the Tauri webview and modern browsers;
  // retry on the astronomically unlikely collision rather than trust the first.
  for (let attempt = 0; attempt < 100; attempt += 1) {
    const id = globalThis.crypto.randomUUID();
    if (!used.has(id)) return id;
  }
  // Deterministic fallback if a host somehow returns repeated UUIDs.
  let counter = existing.length;
  let id = `node-${counter}`;
  while (used.has(id)) {
    counter += 1;
    id = `node-${counter}`;
  }
  return id;
}

export function constructNode(kind: AddNodeKind, id: string): PipelineNode {
  switch (kind) {
    case 'ask-agent': {
      const node: PromptNode = { type: 'prompt', id, prompt: '', agent: '' };
      return node;
    }
    case 'add-session-node': {
      const node: PromptNode = {
        type: 'prompt',
        id,
        prompt: '',
        agent: '',
        sessionRef: { agent: '', sessionId: '' },
      };
      return node;
    }
    case 'run-tool': {
      const node: ToolNode = { type: 'tool', id, toolName: '' };
      return node;
    }
    case 'think': {
      const node: SequentialThinkingNode = {
        type: 'sequentialThinking',
        id,
        topic: '',
        agent: '',
      };
      return node;
    }
    case 'run-workflow': {
      // `type` is intentionally omitted: WorkflowNode.type is optional and
      // defaults to 'workflow'; preserving its absence keeps the round-trip exact.
      const node: WorkflowNode = { id, workflowId: '' };
      return node;
    }
    default: {
      // Exhaustiveness guard — every AddNodeKind is handled above.
      const never: never = kind;
      throw new Error(`Unknown add-node kind: ${String(never)}`);
    }
  }
}

/** Deep-clone a node config for duplicate, assigning a fresh id. */
export function duplicateNode(node: PipelineNode, id: string): PipelineNode {
  // JSON round-trip clone. Unlike `structuredClone`, this reads values through
  // any Vue reactive proxy (the store passes reactive nodes here), so it never
  // throws "could not be cloned" in WebView2. Pipeline nodes are pure JSON data.
  const clone = JSON.parse(JSON.stringify(node)) as Record<string, unknown>;
  clone.id = id;
  return clone as unknown as PipelineNode;
}

// --- validity gate ----------------------------------------------------------

export interface ValidationIssue {
  kind: 'node' | 'edge';
  ref: string;
  message: string;
}

function nodeRequiredIssue(node: PipelineNode): string | null {
  const type = node.type ?? 'workflow';
  switch (type) {
    case 'prompt': {
      const prompt = node as PromptNode;
      if (!prompt.prompt?.trim()) return 'prompt is required';
      if (!prompt.agent?.trim()) return 'agent is required';
      return null;
    }
    case 'sequentialThinking': {
      const think = node as SequentialThinkingNode;
      if (!think.topic?.trim()) return 'topic is required';
      if (!think.agent?.trim()) return 'agent is required';
      return null;
    }
    case 'tool': {
      const tool = node as ToolNode;
      if (!tool.toolName?.trim()) return 'toolName is required';
      return null;
    }
    case 'workflow':
    default: {
      const workflow = node as WorkflowNode;
      if (!workflow.workflowId?.trim()) return 'workflowId is required';
      return null;
    }
  }
}

function hasSchema(node: PipelineNode | undefined, key: 'inputSchema' | 'outputSchema'): boolean {
  if (!node) return false;
  const schema = (node as unknown as Record<string, unknown>)[key];
  return typeof schema === 'object' && schema !== null;
}

// Binding edges REQUIRE explicit `bindings: {from,to}[]` rows. `mapping` is a
// supplementary hint only and can never satisfy a binding edge on its own, so it
// is deliberately excluded from this count.
function edgeBindingRowCount(edge: PipelineEdge): number {
  return Array.isArray(edge.bindings)
    ? edge.bindings.filter((b) => Boolean(b?.from?.trim()) && Boolean(b?.to?.trim())).length
    : 0;
}

function edgeIssue(
  edge: PipelineEdge,
  nodesById: Map<string, PipelineNode>
): string | null {
  const source = nodesById.get(edge.source);
  const target = nodesById.get(edge.target);
  if (!source) return `unknown source node "${edge.source}"`;
  if (!target) return `unknown target node "${edge.target}"`;
  if (!edge.type) return 'edge type is required';
  if (edge.type === 'conditional') {
    if (!edge.path?.trim()) return 'conditional edge needs a path';
    if (edge.equals === undefined) return 'conditional edge needs an equals value';
  }
  if (edge.mode === 'binding') {
    if (!hasSchema(source, 'outputSchema')) {
      return 'binding edge needs source outputSchema';
    }
    if (!hasSchema(target, 'inputSchema')) {
      return 'binding edge needs target inputSchema';
    }
    if (edgeBindingRowCount(edge) === 0) {
      return 'binding edge needs at least one binding row';
    }
  }
  return null;
}

/** Pre-save validity gate. Empty array ⇒ the definition is safe to persist. */
export function validateDefinition(definition: PipelineDefinition): ValidationIssue[] {
  const issues: ValidationIssue[] = [];
  const nodesById = new Map<string, PipelineNode>();
  for (const node of definition.nodes) {
    nodesById.set(node.id, node);
  }
  for (const node of definition.nodes) {
    const message = nodeRequiredIssue(node);
    if (message) issues.push({ kind: 'node', ref: node.id, message });
  }
  definition.edges.forEach((edge, index) => {
    const message = edgeIssue(edge, nodesById);
    if (message) issues.push({ kind: 'edge', ref: `${edge.source}→${edge.target} #${index}`, message });
  });
  return issues;
}
