// Wire-format ↔ local PipelineNode/PipelineDefinition conversion.

import type {
  PipelineEdge,
  PipelineDefinition,
} from '../lib/pipeline-types-edge';
import type {
  PipelineNode,
} from '../lib/pipeline-types-node';
import type { PipelineView, ViewNode } from '../lib/pipeline-types-view';
import {
  cloneAndExtractAgentConfig,
  cloneInputs,
  cloneNodeAdopt,
  cloneSessionRef,
  mergeLegacyExecutionFields,
} from './pipeline-view-transforms-clone';

function toLocalNode(node: ViewNode): PipelineNode {
  switch (node.type) {
    case 'prompt': {
      const agentConfig = mergeLegacyExecutionFields(node);
      return {
        id: node.nodeId,
        type: 'prompt',
        ...(node.displayLabel ? { displayLabel: node.displayLabel } : {}),
        prompt: node.prompt,
        agent: node.agent,
        ...(node.sessionRef ? { sessionRef: cloneSessionRef(node.sessionRef) } : {}),
        ...(node.toolMode ? { toolMode: node.toolMode } : {}),
        ...(node.adopt ? { adopt: cloneNodeAdopt(node.adopt) } : {}),
        ...(node.requiresRole ? { requiresRole: node.requiresRole } : {}),
        ...(Array.isArray(node.allowedTools) ? { allowedTools: [...node.allowedTools] } : {}),
        ...(typeof node.maxTurns === 'number' ? { maxTurns: node.maxTurns } : {}),
        ...(node.cwd ? { cwd: node.cwd } : {}),
        ...(typeof node.timeout === 'number' ? { timeout: node.timeout } : {}),
        ...(agentConfig ? { agentConfig } : {}),
        ...(node.inputSchema ? { inputSchema: { ...node.inputSchema } } : {}),
        ...(node.outputSchema ? { outputSchema: { ...node.outputSchema } } : {}),
      };
    }
    case 'sequentialThinking': {
      const agentConfig = mergeLegacyExecutionFields(node);
      return {
        id: node.nodeId,
        type: 'sequentialThinking',
        ...(node.displayLabel ? { displayLabel: node.displayLabel } : {}),
        topic: node.topic,
        agent: node.agent,
        ...(typeof node.totalThoughts === 'number' ? { totalThoughts: node.totalThoughts } : {}),
        ...(node.analysisType ? { analysisType: node.analysisType } : {}),
        ...(node.adopt ? { adopt: cloneNodeAdopt(node.adopt) } : {}),
        ...(node.requiresRole ? { requiresRole: node.requiresRole } : {}),
        ...(Array.isArray(node.allowedTools) ? { allowedTools: [...node.allowedTools] } : {}),
        ...(typeof node.maxTurns === 'number' ? { maxTurns: node.maxTurns } : {}),
        ...(node.cwd ? { cwd: node.cwd } : {}),
        ...(typeof node.timeout === 'number' ? { timeout: node.timeout } : {}),
        ...(agentConfig ? { agentConfig } : {}),
        ...(node.inputSchema ? { inputSchema: { ...node.inputSchema } } : {}),
        ...(node.outputSchema ? { outputSchema: { ...node.outputSchema } } : {}),
      };
    }
    case 'workflow':
    case undefined: {
      const agentConfig = cloneAndExtractAgentConfig(node.agentConfig);
      const client = node.acpClient;
      return {
        id: node.nodeId,
        type: 'workflow',
        ...(node.displayLabel ? { displayLabel: node.displayLabel } : {}),
        workflowId: node.workflowId,
        ...(client ? { acpClient: client } : {}),
        ...(node.adopt ? { adopt: cloneNodeAdopt(node.adopt) } : {}),
        ...(node.requiresRole ? { requiresRole: node.requiresRole } : {}),
        ...(node.prompt ? { prompt: node.prompt } : {}),
        ...(node.cwd ? { cwd: node.cwd } : {}),
        ...(typeof node.timeout === 'number' ? { timeout: node.timeout } : {}),
        ...(agentConfig ? { agentConfig } : {}),
        ...(node.inputSchema ? { inputSchema: { ...node.inputSchema } } : {}),
        ...(node.outputSchema ? { outputSchema: { ...node.outputSchema } } : {}),
      };
    }
    case 'tool': {
      const agentConfig = mergeLegacyExecutionFields(node);
      return {
        id: node.nodeId,
        type: 'tool',
        ...(node.displayLabel ? { displayLabel: node.displayLabel } : {}),
        toolName: node.toolName,
        ...(node.agent ? { agent: node.agent } : {}),
        ...(node.toolArgs ? { toolArgs: { ...node.toolArgs } } : {}),
        ...(Array.isArray(node.allowedTools) ? { allowedTools: [...node.allowedTools] } : {}),
        ...(typeof node.maxTurns === 'number' ? { maxTurns: node.maxTurns } : {}),
        ...(node.cwd ? { cwd: node.cwd } : {}),
        ...(typeof node.timeout === 'number' ? { timeout: node.timeout } : {}),
        ...(agentConfig ? { agentConfig } : {}),
        ...(node.inputSchema ? { inputSchema: { ...node.inputSchema } } : {}),
        ...(node.outputSchema ? { outputSchema: { ...node.outputSchema } } : {}),
      };
    }
  }
}

function toLocalEdge(edge: PipelineEdge): PipelineEdge {
  const bindings = edge.bindings ?? Object.entries(edge.mapping ?? {}).map(([to, from]) => ({
    to: to.startsWith('inputs.') ? to.slice('inputs.'.length) : to,
    from,
  }));
  return bindings.length > 0 ? { ...edge, bindings } : { ...edge };
}

export function toLocalDefinition(view: PipelineView): PipelineDefinition {
  return {
    ...(view.templateId ? { templateId: view.templateId } : {}),
    ...(view.inputs ? { inputs: cloneInputs(view.inputs) } : {}),
    ...(view.inputSchema ? { inputSchema: { ...view.inputSchema } } : {}),
    nodes: view.nodes.map(toLocalNode),
    edges: view.edges.map(toLocalEdge),
  };
}

function serializeNode(node: PipelineNode): PipelineNode {
  switch (node.type) {
    case 'prompt': {
      const agentConfig = mergeLegacyExecutionFields(node);
      return {
        id: node.id,
        type: 'prompt',
        ...(node.displayLabel ? { displayLabel: node.displayLabel } : {}),
        prompt: node.prompt,
        agent: node.agent,
        ...(node.sessionRef ? { sessionRef: cloneSessionRef(node.sessionRef) } : {}),
        ...(node.toolMode ? { toolMode: node.toolMode } : {}),
        ...(node.adopt ? { adopt: cloneNodeAdopt(node.adopt) } : {}),
        ...(node.requiresRole ? { requiresRole: node.requiresRole } : {}),
        ...(Array.isArray(node.allowedTools) ? { allowedTools: [...node.allowedTools] } : {}),
        ...(typeof node.maxTurns === 'number' ? { maxTurns: node.maxTurns } : {}),
        ...(node.cwd ? { cwd: node.cwd } : {}),
        ...(typeof node.timeout === 'number' ? { timeout: node.timeout } : {}),
        ...(agentConfig ? { agentConfig } : {}),
        ...(node.inputSchema ? { inputSchema: { ...node.inputSchema } } : {}),
        ...(node.outputSchema ? { outputSchema: { ...node.outputSchema } } : {}),
      };
    }
    case 'sequentialThinking': {
      const agentConfig = mergeLegacyExecutionFields(node);
      return {
        id: node.id,
        type: 'sequentialThinking',
        ...(node.displayLabel ? { displayLabel: node.displayLabel } : {}),
        topic: node.topic,
        agent: node.agent,
        ...(typeof node.totalThoughts === 'number' ? { totalThoughts: node.totalThoughts } : {}),
        ...(node.analysisType ? { analysisType: node.analysisType } : {}),
        ...(node.adopt ? { adopt: cloneNodeAdopt(node.adopt) } : {}),
        ...(node.requiresRole ? { requiresRole: node.requiresRole } : {}),
        ...(Array.isArray(node.allowedTools) ? { allowedTools: [...node.allowedTools] } : {}),
        ...(typeof node.maxTurns === 'number' ? { maxTurns: node.maxTurns } : {}),
        ...(node.cwd ? { cwd: node.cwd } : {}),
        ...(typeof node.timeout === 'number' ? { timeout: node.timeout } : {}),
        ...(agentConfig ? { agentConfig } : {}),
        ...(node.inputSchema ? { inputSchema: { ...node.inputSchema } } : {}),
        ...(node.outputSchema ? { outputSchema: { ...node.outputSchema } } : {}),
      };
    }
    case 'workflow':
    case undefined: {
      const agentConfig = cloneAndExtractAgentConfig(node.agentConfig);
      return {
        id: node.id,
        type: 'workflow',
        ...(node.displayLabel ? { displayLabel: node.displayLabel } : {}),
        workflowId: node.workflowId,
        ...(node.acpClient ? { acpClient: node.acpClient } : {}),
        ...(node.adopt ? { adopt: cloneNodeAdopt(node.adopt) } : {}),
        ...(node.requiresRole ? { requiresRole: node.requiresRole } : {}),
        ...(node.prompt ? { prompt: node.prompt } : {}),
        ...(node.cwd ? { cwd: node.cwd } : {}),
        ...(agentConfig ? { agentConfig } : {}),
        ...(node.inputSchema ? { inputSchema: { ...node.inputSchema } } : {}),
        ...(node.outputSchema ? { outputSchema: { ...node.outputSchema } } : {}),
      };
    }
    case 'tool': {
      const agentConfig = mergeLegacyExecutionFields(node);
      return {
        id: node.id,
        type: 'tool',
        ...(node.displayLabel ? { displayLabel: node.displayLabel } : {}),
        toolName: node.toolName,
        ...(node.agent ? { agent: node.agent } : {}),
        ...(node.toolArgs ? { toolArgs: { ...node.toolArgs } } : {}),
        ...(Array.isArray(node.allowedTools) ? { allowedTools: [...node.allowedTools] } : {}),
        ...(typeof node.maxTurns === 'number' ? { maxTurns: node.maxTurns } : {}),
        ...(node.cwd ? { cwd: node.cwd } : {}),
        ...(typeof node.timeout === 'number' ? { timeout: node.timeout } : {}),
        ...(agentConfig ? { agentConfig } : {}),
        ...(node.inputSchema ? { inputSchema: { ...node.inputSchema } } : {}),
        ...(node.outputSchema ? { outputSchema: { ...node.outputSchema } } : {}),
      };
    }
  }
}

export function serializePipelineDefinition(definition: PipelineDefinition): PipelineDefinition {
  return {
    ...(definition.templateId ? { templateId: definition.templateId } : {}),
    ...(definition.inputs ? { inputs: cloneInputs(definition.inputs) } : {}),
    ...(definition.inputSchema ? { inputSchema: { ...definition.inputSchema } } : {}),
    nodes: definition.nodes.map(serializeNode),
    edges: definition.edges.map((edge) => ({ ...edge })),
  };
}
