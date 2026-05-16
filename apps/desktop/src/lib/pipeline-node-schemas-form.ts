import type {
  PipelineNode,
  PromptNode,
  SequentialThinkingNode,
  ToolNode,
  WorkflowNode,
} from './pipeline-types-node';
import { getNodeType } from './pipeline-types-node';
import {
  OPTIONAL_NONE_VALUE,
  parseToolArgs,
  readOptionalInteger,
  readOptionalSessionRef,
  readOptionalString,
  readRequiredString,
} from './pipeline-node-schemas-defs';

export function getPipelineNodeSchemaFormValue(node: PipelineNode): Record<string, unknown> {
  switch (getNodeType(node)) {
    case 'workflow': {
      const workflowNode = node as WorkflowNode;
      return {
        workflowId: workflowNode.workflowId,
        acpClient: workflowNode.acpClient ?? OPTIONAL_NONE_VALUE,
        prompt: workflowNode.prompt ?? '',
      };
    }
    case 'prompt': {
      const promptNode = node as PromptNode;
      return {
        agent: promptNode.agent,
        prompt: promptNode.prompt,
        ...(promptNode.sessionRef
          ? {
              sessionRef: {
                agent: promptNode.sessionRef.agent,
                sessionId: promptNode.sessionRef.sessionId,
              },
            }
          : {}),
      };
    }
    case 'sequentialThinking': {
      const thinkingNode = node as SequentialThinkingNode;
      return {
        topic: thinkingNode.topic,
        agent: thinkingNode.agent,
        totalThoughts: thinkingNode.totalThoughts,
        analysisType: thinkingNode.analysisType ?? '',
      };
    }
    case 'tool': {
      const toolNode = node as ToolNode;
      return {
        toolName: toolNode.toolName,
        agent: toolNode.agent ?? OPTIONAL_NONE_VALUE,
        toolArgs: toolNode.toolArgs ? JSON.stringify(toolNode.toolArgs, null, 2) : '',
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
      const sessionRef = readOptionalSessionRef(nextValue.sessionRef);
      const agentChanged = promptNode.agent !== agent;
      if (sessionRef && sessionRef.agent !== agent) {
        throw new Error(
          `Session reference agent ('${sessionRef.agent}') must match node agent ('${agent}').`,
        );
      }
      const {
        agent: _agent,
        agentConfig: _agentConfig,
        model: _model,
        prompt: _prompt,
        sessionRef: _sessionRef,
        thoughtLevel: _thoughtLevel,
        ...rest
      } = promptNode;

      return {
        ...rest,
        type: 'prompt',
        agent,
        prompt,
        ...(sessionRef ? { sessionRef } : {}),
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
