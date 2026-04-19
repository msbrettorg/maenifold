import type { JsonSchema } from '@jsonforms/core';
import type { PipelineNode } from './pipeline-types';
import { getNodeType } from './pipeline-types';
import {
  applyEnumValues,
  pipelineNodeSchemas,
  type PipelineNodeSchemaOptions,
} from './pipeline-node-schemas-defs';

export type {
  PipelineNodeSchemaOptions,
  ObjectSchema,
  WorkflowSchemaKeys,
  PromptSchemaKeys,
  SequentialThinkingSchemaKeys,
  ToolSchemaKeys,
} from './pipeline-node-schemas-defs';
export { pipelineNodeSchemas } from './pipeline-node-schemas-defs';
export {
  getPipelineNodeSchemaFormValue,
  applyPipelineNodeSchemaFormValue,
} from './pipeline-node-schemas-form';

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
