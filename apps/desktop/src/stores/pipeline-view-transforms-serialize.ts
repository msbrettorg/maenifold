// Wire-format ↔ local PipelineNode/PipelineDefinition conversion.

import type {
  PipelineDefinition,
  PipelineNode,
  PipelineView,
  ViewNode,
} from '../lib/pipeline-types';
import {
  cloneAndExtractAgentConfig,
  cloneInputs,
  cloneNodeAdopt,
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
        ...(node.toolMode ? { toolMode: node.toolMode } : {}),
        ...(node.adopt ? { adopt: cloneNodeAdopt(node.adopt) } : {}),
        ...(node.requiresRole ? { requiresRole: node.requiresRole } : {}),
        ...(Array.isArray(node.allowedTools) ? { allowedTools: [...node.allowedTools] } : {}),
        ...(typeof node.maxTurns === 'number' ? { maxTurns: node.maxTurns } : {}),
        ...(node.cwd ? { cwd: node.cwd } : {}),
        ...(typeof node.timeout === 'number' ? { timeout: node.timeout } : {}),
        ...(agentConfig ? { agentConfig } : {}),
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
      };
    }
  }
}

export function toLocalDefinition(view: PipelineView): PipelineDefinition {
  return {
    ...(view.templateId ? { templateId: view.templateId } : {}),
    ...(view.inputs ? { inputs: cloneInputs(view.inputs) } : {}),
    nodes: view.nodes.map(toLocalNode),
    edges: view.edges.map((edge) => ({ ...edge })),
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
        ...(node.toolMode ? { toolMode: node.toolMode } : {}),
        ...(node.adopt ? { adopt: cloneNodeAdopt(node.adopt) } : {}),
        ...(node.requiresRole ? { requiresRole: node.requiresRole } : {}),
        ...(Array.isArray(node.allowedTools) ? { allowedTools: [...node.allowedTools] } : {}),
        ...(typeof node.maxTurns === 'number' ? { maxTurns: node.maxTurns } : {}),
        ...(node.cwd ? { cwd: node.cwd } : {}),
        ...(typeof node.timeout === 'number' ? { timeout: node.timeout } : {}),
        ...(agentConfig ? { agentConfig } : {}),
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
      };
    }
  }
}

export function serializePipelineDefinition(definition: PipelineDefinition): PipelineDefinition {
  return {
    ...(definition.templateId ? { templateId: definition.templateId } : {}),
    ...(definition.inputs ? { inputs: cloneInputs(definition.inputs) } : {}),
    nodes: definition.nodes.map(serializeNode),
    edges: definition.edges.map((edge) => ({ ...edge })),
  };
}
