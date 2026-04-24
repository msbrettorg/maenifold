import type { JsonSchema } from '@jsonforms/core';
import type {
  PromptNode,
  SessionRef,
  SequentialThinkingNode,
  ToolNode,
  WorkflowNode,
} from './pipeline-types';

export type ObjectSchema<PropertyKeys extends string> = JsonSchema & {
  properties: Partial<Record<PropertyKeys, JsonSchema>>;
  required?: PropertyKeys[];
  type: 'object';
};

export type WorkflowSchemaKeys = keyof Pick<WorkflowNode, 'workflowId' | 'acpClient' | 'prompt'>;
export type PromptSchemaKeys = keyof Pick<PromptNode, 'agent' | 'prompt' | 'sessionRef'>;
export type SequentialThinkingSchemaKeys = keyof Pick<
  SequentialThinkingNode,
  'agent' | 'analysisType' | 'topic' | 'totalThoughts'
>;
export type ToolSchemaKeys = keyof Pick<ToolNode, 'agent' | 'toolArgs' | 'toolName'>;

export const OPTIONAL_NONE_VALUE = '__none__';

export interface PipelineNodeSchemaOptions {
  agentNames?: string[];
  workflows?: string[];
}

export const pipelineNodeSchemas: {
  prompt: ObjectSchema<PromptSchemaKeys>;
  sequentialThinking: ObjectSchema<SequentialThinkingSchemaKeys>;
  tool: ObjectSchema<ToolSchemaKeys>;
  workflow: ObjectSchema<WorkflowSchemaKeys>;
} = {
  workflow: {
    type: 'object',
    required: ['workflowId'],
    properties: {
      workflowId: { title: 'Workflow ID', type: 'string' },
      acpClient: { title: 'ACP client', type: 'string' },
      prompt: { title: 'Prompt', type: 'string', format: 'textarea' },
    },
  },
  prompt: {
    type: 'object',
    required: ['agent', 'prompt'],
    properties: {
      agent: { title: 'Agent', type: 'string' },
      prompt: { title: 'Prompt template', type: 'string', format: 'textarea' },
      sessionRef: {
        title: 'Session reference',
        type: 'object',
        properties: {
          agent: { title: 'Session agent', type: 'string' },
          sessionId: { title: 'Session ID', type: 'string' },
        },
        required: ['agent', 'sessionId'],
      },
    },
  },
  sequentialThinking: {
    type: 'object',
    required: ['agent', 'topic'],
    properties: {
      topic: { title: 'Topic', type: 'string', format: 'textarea' },
      agent: { title: 'Agent', type: 'string' },
      totalThoughts: { title: 'Total thoughts', type: 'integer', minimum: 1 },
      analysisType: { title: 'Analysis type', type: 'string' },
    },
  },
  tool: {
    type: 'object',
    required: ['toolName'],
    properties: {
      toolName: { title: 'Tool name', type: 'string' },
      agent: { title: 'Agent', type: 'string' },
      toolArgs: {
        title: 'Tool arguments',
        description: 'JSON object passed to the tool.',
        type: 'string',
        format: 'textarea',
      },
    },
  },
};

export function applyEnumValues(
  schema: JsonSchema,
  key: string,
  values: string[],
  options: { optional: boolean },
): void {
  if (!schema || typeof schema !== 'object' || !('properties' in schema)) {
    return;
  }

  const properties = schema.properties as Record<string, JsonSchema | undefined>;
  const property = properties[key];

  if (!property || typeof property !== 'object') {
    return;
  }

  const normalizedValues = values
    .map((value) => value.trim())
    .filter((value) => value.length > 0);

  if (normalizedValues.length === 0) {
    return;
  }

  properties[key] = {
    ...property,
    enum: options.optional
      ? [OPTIONAL_NONE_VALUE, ...normalizedValues]
      : normalizedValues,
  } as JsonSchema;
}

export function readRequiredString(value: unknown, fieldName: string): string {
  const normalized = readOptionalString(value);
  if (!normalized) {
    throw new Error(`${fieldName} is required.`);
  }
  return normalized;
}

export function readOptionalString(value: unknown): string | undefined {
  if (typeof value !== 'string') {
    return undefined;
  }

  if (value === OPTIONAL_NONE_VALUE) {
    return undefined;
  }

  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : undefined;
}

export function readOptionalInteger(value: unknown, fieldName: string): number | undefined {
  if (value == null || value === '') {
    return undefined;
  }

  const resolved = typeof value === 'number' ? value : Number(value);
  if (!Number.isFinite(resolved) || !Number.isInteger(resolved) || resolved < 1) {
    throw new Error(`${fieldName} must be a whole number greater than zero.`);
  }

  return resolved;
}

export function parseToolArgs(value: unknown): Record<string, unknown> | undefined {
  if (typeof value !== 'string') {
    return undefined;
  }

  const trimmed = value.trim();
  if (trimmed.length === 0) {
    return undefined;
  }

  let parsed: unknown;
  try {
    parsed = JSON.parse(trimmed);
  } catch {
    throw new Error('Tool arguments must be valid JSON.');
  }

  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
    throw new Error('Tool arguments must be a JSON object.');
  }

  return parsed as Record<string, unknown>;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

export function readOptionalSessionRef(value: unknown): SessionRef | undefined {
  if (value == null) {
    return undefined;
  }

  if (!isRecord(value)) {
    throw new Error('Session reference must be an object.');
  }

  const agent = readOptionalString(value.agent);
  const sessionId = readOptionalString(value.sessionId);

  if (!agent && !sessionId) {
    return undefined;
  }

  if (!agent || !sessionId) {
    throw new Error('Session reference requires both agent and sessionId.');
  }

  return { agent, sessionId };
}
