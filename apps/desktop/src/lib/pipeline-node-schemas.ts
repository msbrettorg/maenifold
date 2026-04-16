import type { JsonSchema } from '@jsonforms/core';
import type {
  PipelineNode,
  PromptNode,
  SequentialThinkingNode,
  ToolNode,
  WorkflowNode,
} from './pipeline-types';
import { getNodeType } from './pipeline-types';

type ObjectSchema<PropertyKeys extends string> = JsonSchema & {
  properties: Partial<Record<PropertyKeys, JsonSchema>>;
  required?: PropertyKeys[];
  type: 'object';
};

type WorkflowSchemaKeys = keyof Pick<WorkflowNode, 'workflowId' | 'acpClient' | 'prompt'>;
type PromptSchemaKeys = keyof Pick<PromptNode, 'agent' | 'prompt'>;
type SequentialThinkingSchemaKeys = keyof Pick<
  SequentialThinkingNode,
  'agent' | 'analysisType' | 'topic' | 'totalThoughts'
>;
type ToolSchemaKeys = keyof Pick<ToolNode, 'agent' | 'toolArgs' | 'toolName'>;

const OPTIONAL_NONE_VALUE = '__none__';

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
      workflowId: {
        title: 'Workflow ID',
        type: 'string',
      },
      acpClient: {
        title: 'ACP client',
        type: 'string',
      },
      prompt: {
        title: 'Prompt',
        type: 'string',
        format: 'textarea',
      },
    },
  },
  prompt: {
    type: 'object',
    required: ['agent', 'prompt'],
    properties: {
      agent: {
        title: 'Agent',
        type: 'string',
      },
      prompt: {
        title: 'Prompt template',
        type: 'string',
        format: 'textarea',
      },
    },
  },
  sequentialThinking: {
    type: 'object',
    required: ['agent', 'topic'],
    properties: {
      topic: {
        title: 'Topic',
        type: 'string',
        format: 'textarea',
      },
      agent: {
        title: 'Agent',
        type: 'string',
      },
      totalThoughts: {
        title: 'Total thoughts',
        type: 'integer',
        minimum: 1,
      },
      analysisType: {
        title: 'Analysis type',
        type: 'string',
      },
    },
  },
  tool: {
    type: 'object',
    required: ['toolName'],
    properties: {
      toolName: {
        title: 'Tool name',
        type: 'string',
      },
      agent: {
        title: 'Agent',
        type: 'string',
      },
      toolArgs: {
        title: 'Tool arguments',
        description: 'JSON object passed to the tool.',
        type: 'string',
        format: 'textarea',
      },
    },
  },
};

export function getPipelineNodeSchema(
  node: PipelineNode,
  options: PipelineNodeSchemaOptions = {},
): JsonSchema {
  const type = getNodeType(node);
  const schema = structuredClone(pipelineNodeSchemas[type]);

  if (type === 'workflow') {
    applyEnumValues(schema, 'workflowId', options.workflows ?? [], { optional: false });
    applyEnumValues(schema, 'acpClient', options.agentNames ?? [], { optional: true });
    return schema;
  }

  applyEnumValues(schema, 'agent', options.agentNames ?? [], {
    optional: type === 'tool',
  });
  return schema;
}

export function getPipelineNodeSchemaFormValue(node: PipelineNode): Record<string, unknown> {
  switch (getNodeType(node)) {
    case 'workflow':
      {
        const workflowNode = node as WorkflowNode;
      return {
        workflowId: workflowNode.workflowId,
        acpClient: workflowNode.acpClient ?? OPTIONAL_NONE_VALUE,
        prompt: workflowNode.prompt ?? '',
      };
      }
    case 'prompt':
      {
        const promptNode = node as PromptNode;
      return {
        agent: promptNode.agent,
        prompt: promptNode.prompt,
      };
      }
    case 'sequentialThinking':
      {
        const thinkingNode = node as SequentialThinkingNode;
      return {
        topic: thinkingNode.topic,
        agent: thinkingNode.agent,
        totalThoughts: thinkingNode.totalThoughts,
        analysisType: thinkingNode.analysisType ?? '',
      };
      }
    case 'tool':
      {
        const toolNode = node as ToolNode;
      return {
        toolName: toolNode.toolName,
        agent: toolNode.agent ?? OPTIONAL_NONE_VALUE,
        toolArgs: toolNode.toolArgs
          ? JSON.stringify(toolNode.toolArgs, null, 2)
          : '',
      };
      }
  }
}

export function applyPipelineNodeSchemaFormValue(
  node: PipelineNode,
  nextValue: Record<string, unknown>,
): PipelineNode {
  switch (getNodeType(node)) {
    case 'workflow': {
      const workflowNode = node as WorkflowNode;
      const workflowId = readRequiredString(nextValue.workflowId, 'Workflow ID');
      const acpClient = readOptionalString(nextValue.acpClient);
      const prompt = readOptionalString(nextValue.prompt);
      const agentChanged = (workflowNode.acpClient ?? undefined) !== acpClient;
      const {
        acpClient: _acpClient,
        agentConfig: _agentConfig,
        model: _model,
        prompt: _prompt,
        thoughtLevel: _thoughtLevel,
        workflowId: _workflowId,
        ...rest
      } = workflowNode;

      return {
        ...rest,
        type: 'workflow',
        workflowId,
        ...(acpClient ? { acpClient } : {}),
        ...(prompt ? { prompt } : {}),
        ...(!agentChanged && workflowNode.agentConfig ? { agentConfig: workflowNode.agentConfig } : {}),
        ...(!agentChanged && workflowNode.model ? { model: workflowNode.model } : {}),
        ...(!agentChanged && workflowNode.thoughtLevel ? { thoughtLevel: workflowNode.thoughtLevel } : {}),
      };
    }
    case 'prompt': {
      const promptNode = node as PromptNode;
      const agent = readRequiredString(nextValue.agent, 'Agent');
      const prompt = readRequiredString(nextValue.prompt, 'Prompt template');
      const agentChanged = promptNode.agent !== agent;
      const {
        agent: _agent,
        agentConfig: _agentConfig,
        model: _model,
        prompt: _prompt,
        thoughtLevel: _thoughtLevel,
        ...rest
      } = promptNode;

      return {
        ...rest,
        type: 'prompt',
        agent,
        prompt,
        ...(!agentChanged && promptNode.agentConfig ? { agentConfig: promptNode.agentConfig } : {}),
        ...(!agentChanged && promptNode.model ? { model: promptNode.model } : {}),
        ...(!agentChanged && promptNode.thoughtLevel ? { thoughtLevel: promptNode.thoughtLevel } : {}),
      };
    }
    case 'sequentialThinking': {
      const thinkingNode = node as SequentialThinkingNode;
      const topic = readRequiredString(nextValue.topic, 'Topic');
      const agent = readRequiredString(nextValue.agent, 'Agent');
      const totalThoughts = readOptionalInteger(nextValue.totalThoughts, 'Total thoughts');
      const analysisType = readOptionalString(nextValue.analysisType);
      const agentChanged = thinkingNode.agent !== agent;
      const {
        agent: _agent,
        agentConfig: _agentConfig,
        analysisType: _analysisType,
        model: _model,
        thoughtLevel: _thoughtLevel,
        topic: _topic,
        totalThoughts: _totalThoughts,
        ...rest
      } = thinkingNode;

      return {
        ...rest,
        type: 'sequentialThinking',
        topic,
        agent,
        ...(typeof totalThoughts === 'number' ? { totalThoughts } : {}),
        ...(analysisType ? { analysisType } : {}),
        ...(!agentChanged && thinkingNode.agentConfig ? { agentConfig: thinkingNode.agentConfig } : {}),
        ...(!agentChanged && thinkingNode.model ? { model: thinkingNode.model } : {}),
        ...(!agentChanged && thinkingNode.thoughtLevel ? { thoughtLevel: thinkingNode.thoughtLevel } : {}),
      };
    }
    case 'tool': {
      const toolNode = node as ToolNode;
      const toolName = readRequiredString(nextValue.toolName, 'Tool name');
      const agent = readOptionalString(nextValue.agent);
      const toolArgs = parseToolArgs(nextValue.toolArgs);
      const agentChanged = (toolNode.agent ?? undefined) !== agent;
      const {
        agent: _agent,
        agentConfig: _agentConfig,
        model: _model,
        thoughtLevel: _thoughtLevel,
        toolArgs: _toolArgs,
        toolName: _toolName,
        ...rest
      } = toolNode;

      return {
        ...rest,
        type: 'tool',
        toolName,
        ...(agent ? { agent } : {}),
        ...(toolArgs ? { toolArgs } : {}),
        ...(!agentChanged && toolNode.agentConfig ? { agentConfig: toolNode.agentConfig } : {}),
        ...(!agentChanged && toolNode.model ? { model: toolNode.model } : {}),
        ...(!agentChanged && toolNode.thoughtLevel ? { thoughtLevel: toolNode.thoughtLevel } : {}),
      };
    }
  }
}

function applyEnumValues(
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

function readRequiredString(value: unknown, fieldName: string): string {
  const normalized = readOptionalString(value);
  if (!normalized) {
    throw new Error(`${fieldName} is required.`);
  }
  return normalized;
}

function readOptionalString(value: unknown): string | undefined {
  if (typeof value !== 'string') {
    return undefined;
  }

  if (value === OPTIONAL_NONE_VALUE) {
    return undefined;
  }

  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : undefined;
}

function readOptionalInteger(value: unknown, fieldName: string): number | undefined {
  if (value == null || value === '') {
    return undefined;
  }

  const resolved = typeof value === 'number' ? value : Number(value);
  if (!Number.isFinite(resolved) || !Number.isInteger(resolved) || resolved < 1) {
    throw new Error(`${fieldName} must be a whole number greater than zero.`);
  }

  return resolved;
}

function parseToolArgs(value: unknown): Record<string, unknown> | undefined {
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
