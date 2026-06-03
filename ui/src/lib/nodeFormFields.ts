// Field descriptors for the design-mode node config form. Declaring the per-type
// fields as data (rather than bespoke template per type) keeps NodeConfigForm.vue
// compact and under the LOC cap, and makes the per-type required-field set
// (which blocks Save) explicit and testable.

import type { PipelineNodeType } from './pipeline-types';

export type FieldKind = 'text' | 'textarea' | 'number' | 'boolean' | 'select' | 'stringlist' | 'json';

export interface FieldDef {
  // Dot path into the node object (one level of nesting supported, e.g.
  // `sessionRef.agent`).
  path: string;
  label: string;
  kind: FieldKind;
  required?: boolean;
  options?: string[];
  placeholder?: string;
}

export interface NodeFormFields {
  required: FieldDef[];
  optional: FieldDef[];
}

const JOIN_POLICY_OPTIONS = ['all', 'any', 'first'];

// Advanced execution controls present on every node-definition variant.
const BASE_OPTIONAL: FieldDef[] = [
  { path: 'displayLabel', label: 'Display label', kind: 'text' },
  { path: 'joinPolicy', label: 'Join policy', kind: 'select', options: JOIN_POLICY_OPTIONS },
  { path: 'retryMaxAttempts', label: 'Retry max attempts', kind: 'number' },
  { path: 'retryBackoffMinMs', label: 'Retry backoff min (ms)', kind: 'number' },
  { path: 'retryBackoffMaxMs', label: 'Retry backoff max (ms)', kind: 'number' },
  { path: 'retryJitter', label: 'Retry jitter', kind: 'boolean' },
];

const SCHEMA_OPTIONAL: FieldDef[] = [
  { path: 'inputSchema', label: 'Input schema (JSON)', kind: 'json' },
  { path: 'outputSchema', label: 'Output schema (JSON)', kind: 'json' },
];

const AGENT_CONFIG: FieldDef = { path: 'agentConfig', label: 'Agent config (JSON)', kind: 'json' };

function promptFields(hasSessionRef: boolean): NodeFormFields {
  const optional: FieldDef[] = [
    { path: 'model', label: 'Model', kind: 'text' },
    { path: 'thoughtLevel', label: 'Thought level', kind: 'text' },
    { path: 'requiresRole', label: 'Requires role', kind: 'text' },
    { path: 'toolMode', label: 'Tool mode', kind: 'text' },
    { path: 'allowedTools', label: 'Allowed tools', kind: 'stringlist' },
    { path: 'maxTurns', label: 'Max turns', kind: 'number' },
    { path: 'cwd', label: 'Working dir (cwd)', kind: 'text' },
    { path: 'timeout', label: 'Timeout (s)', kind: 'number' },
    AGENT_CONFIG,
    ...BASE_OPTIONAL,
    ...SCHEMA_OPTIONAL,
  ];
  if (hasSessionRef) {
    optional.unshift(
      { path: 'sessionRef.agent', label: 'Session agent', kind: 'text' },
      { path: 'sessionRef.sessionId', label: 'Session id', kind: 'text' }
    );
  }
  return {
    required: [
      { path: 'prompt', label: 'Prompt', kind: 'textarea', required: true },
      { path: 'agent', label: 'Agent', kind: 'text', required: true },
    ],
    optional,
  };
}

function workflowFields(): NodeFormFields {
  return {
    required: [{ path: 'workflowId', label: 'Workflow id', kind: 'text', required: true }],
    optional: [
      { path: 'acpClient', label: 'ACP client', kind: 'text' },
      { path: 'prompt', label: 'Prompt', kind: 'textarea' },
      { path: 'requiresRole', label: 'Requires role', kind: 'text' },
      { path: 'model', label: 'Model', kind: 'text' },
      { path: 'thoughtLevel', label: 'Thought level', kind: 'text' },
      { path: 'cwd', label: 'Working dir (cwd)', kind: 'text' },
      { path: 'timeout', label: 'Timeout (s)', kind: 'number' },
      AGENT_CONFIG,
      ...BASE_OPTIONAL,
      ...SCHEMA_OPTIONAL,
    ],
  };
}

function thinkingFields(): NodeFormFields {
  return {
    required: [
      { path: 'topic', label: 'Topic', kind: 'text', required: true },
      { path: 'agent', label: 'Agent', kind: 'text', required: true },
    ],
    optional: [
      { path: 'totalThoughts', label: 'Total thoughts', kind: 'number' },
      { path: 'analysisType', label: 'Analysis type', kind: 'text' },
      { path: 'model', label: 'Model', kind: 'text' },
      { path: 'thoughtLevel', label: 'Thought level', kind: 'text' },
      { path: 'allowedTools', label: 'Allowed tools', kind: 'stringlist' },
      { path: 'maxTurns', label: 'Max turns', kind: 'number' },
      { path: 'cwd', label: 'Working dir (cwd)', kind: 'text' },
      { path: 'timeout', label: 'Timeout (s)', kind: 'number' },
      AGENT_CONFIG,
      ...BASE_OPTIONAL,
      ...SCHEMA_OPTIONAL,
    ],
  };
}

function toolFields(): NodeFormFields {
  return {
    required: [{ path: 'toolName', label: 'Tool name', kind: 'text', required: true }],
    optional: [
      { path: 'toolArgs', label: 'Tool args (JSON)', kind: 'json' },
      { path: 'agent', label: 'Agent', kind: 'text' },
      { path: 'model', label: 'Model', kind: 'text' },
      { path: 'thoughtLevel', label: 'Thought level', kind: 'text' },
      { path: 'allowedTools', label: 'Allowed tools', kind: 'stringlist' },
      { path: 'maxTurns', label: 'Max turns', kind: 'number' },
      { path: 'cwd', label: 'Working dir (cwd)', kind: 'text' },
      { path: 'timeout', label: 'Timeout (s)', kind: 'number' },
      AGENT_CONFIG,
      ...BASE_OPTIONAL,
      ...SCHEMA_OPTIONAL,
    ],
  };
}

export function fieldsForType(type: PipelineNodeType, hasSessionRef: boolean): NodeFormFields {
  switch (type) {
    case 'prompt':
      return promptFields(hasSessionRef);
    case 'sequentialThinking':
      return thinkingFields();
    case 'tool':
      return toolFields();
    case 'workflow':
    default:
      return workflowFields();
  }
}

// --- path get/set (one level of nesting) -----------------------------------

export function getPath(obj: Record<string, unknown>, path: string): unknown {
  const parts = path.split('.');
  let cursor: unknown = obj;
  for (const part of parts) {
    if (cursor === null || typeof cursor !== 'object') return undefined;
    cursor = (cursor as Record<string, unknown>)[part];
  }
  return cursor;
}

export function setPath(obj: Record<string, unknown>, path: string, value: unknown): void {
  const parts = path.split('.');
  let cursor: Record<string, unknown> = obj;
  for (let i = 0; i < parts.length - 1; i += 1) {
    const part = parts[i];
    const next = cursor[part];
    if (next === null || typeof next !== 'object') {
      cursor[part] = {};
    }
    cursor = cursor[part] as Record<string, unknown>;
  }
  cursor[parts[parts.length - 1]] = value;
}

export function deletePath(obj: Record<string, unknown>, path: string): void {
  const parts = path.split('.');
  let cursor: Record<string, unknown> = obj;
  for (let i = 0; i < parts.length - 1; i += 1) {
    const part = parts[i];
    const next = cursor[part];
    if (next === null || typeof next !== 'object') return;
    cursor = next as Record<string, unknown>;
  }
  delete cursor[parts[parts.length - 1]];
}
