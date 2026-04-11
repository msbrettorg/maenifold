/**
 * Tests for pipeline-composer-flow.ts — specifically the agentConfig round-trip
 * for workflow nodes (T-130.1 red-team blocker).
 *
 * The bug: agentConfig.mode, agentConfig.thoughtLevel, and agentConfig.configOptions
 * were silently dropped when a workflow node passed through:
 *   PipelineNode → toFlowNodeData → FlowNodeData → toPipelineNode → PipelineNode
 * and also when passing through toPipelineNodeFromViewNode.
 */

import { describe, expect, it } from 'vitest';

import type { PipelineNode, ViewNode, WorkflowViewNode } from '../pipeline-types';
import { definitionFromFlow, pipelineNodesToFlow, toPipelineNodeFromViewNode } from '../pipeline-composer-flow';
import { createNode } from '../pipeline-node-factory';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeFlowNodes(nodes: PipelineNode[]) {
  return pipelineNodesToFlow(nodes);
}

// ---------------------------------------------------------------------------
// pipelineNodesToFlow + definitionFromFlow round-trip
// ---------------------------------------------------------------------------

describe('workflow agentConfig round-trip via flow (pipelineNodesToFlow → definitionFromFlow)', () => {
  it('preserves agentConfig.model through the flow round-trip', () => {
    const original = createNode('workflow', {
      id: 'wf-1',
      workflowId: 'my-workflow',
      agentConfig: { model: 'gpt-5' },
    });

    const flowNodes = makeFlowNodes([original]);
    const { nodes } = definitionFromFlow(flowNodes, []);
    const result = nodes[0] as Extract<PipelineNode, { type?: 'workflow' }>;

    expect(result.agentConfig?.model).toBe('gpt-5');
  });

  it('preserves agentConfig.mode through the flow round-trip', () => {
    const original = createNode('workflow', {
      id: 'wf-1',
      workflowId: 'my-workflow',
      agentConfig: { mode: 'workflow' },
    });

    const flowNodes = makeFlowNodes([original]);
    const { nodes } = definitionFromFlow(flowNodes, []);
    const result = nodes[0] as Extract<PipelineNode, { type?: 'workflow' }>;

    expect(result.agentConfig?.mode).toBe('workflow');
  });

  it('preserves agentConfig.thoughtLevel through the flow round-trip', () => {
    const original = createNode('workflow', {
      id: 'wf-1',
      workflowId: 'my-workflow',
      agentConfig: { thoughtLevel: 'high' },
    });

    const flowNodes = makeFlowNodes([original]);
    const { nodes } = definitionFromFlow(flowNodes, []);
    const result = nodes[0] as Extract<PipelineNode, { type?: 'workflow' }>;

    expect(result.agentConfig?.thoughtLevel).toBe('high');
  });

  it('preserves agentConfig.configOptions through the flow round-trip', () => {
    const original = createNode('workflow', {
      id: 'wf-1',
      workflowId: 'my-workflow',
      agentConfig: { configOptions: { stream: true, temperature: 0.7 } },
    });

    const flowNodes = makeFlowNodes([original]);
    const { nodes } = definitionFromFlow(flowNodes, []);
    const result = nodes[0] as Extract<PipelineNode, { type?: 'workflow' }>;

    expect(result.agentConfig?.configOptions).toEqual({ stream: true, temperature: 0.7 });
  });

  it('preserves full agentConfig (model + mode + thoughtLevel + configOptions) in one pass', () => {
    const original = createNode('workflow', {
      id: 'wf-1',
      workflowId: 'my-workflow',
      agentConfig: {
        model: 'claude-opus',
        mode: 'workflow',
        thoughtLevel: 'medium',
        configOptions: { temperature: 0.5 },
      },
    });

    const flowNodes = makeFlowNodes([original]);
    const { nodes } = definitionFromFlow(flowNodes, []);
    const result = nodes[0] as Extract<PipelineNode, { type?: 'workflow' }>;

    expect(result.agentConfig).toEqual({
      model: 'claude-opus',
      mode: 'workflow',
      thoughtLevel: 'medium',
      configOptions: { temperature: 0.5 },
    });
  });

  it('produces no agentConfig when the original workflow node has none', () => {
    const original = createNode('workflow', {
      id: 'wf-bare',
      workflowId: 'bare-workflow',
    });

    const flowNodes = makeFlowNodes([original]);
    const { nodes } = definitionFromFlow(flowNodes, []);
    const result = nodes[0] as Extract<PipelineNode, { type?: 'workflow' }>;

    expect(result.agentConfig).toBeUndefined();
  });

  it('does not affect prompt node round-trip (no agentConfig regression)', () => {
    const original = createNode('prompt', {
      id: 'prompt-1',
      prompt: 'hello',
      agent: 'architect',
      model: 'gpt-4',
    });

    const flowNodes = makeFlowNodes([original]);
    const { nodes } = definitionFromFlow(flowNodes, []);
    const result = nodes[0] as Extract<PipelineNode, { type: 'prompt' }>;

    expect(result.model).toBe('gpt-4');
    expect(result.prompt).toBe('hello');
    expect(result.agent).toBe('architect');
  });
});

// ---------------------------------------------------------------------------
// toPipelineNodeFromViewNode — workflow agentConfig merge
// ---------------------------------------------------------------------------

describe('workflow agentConfig round-trip via toPipelineNodeFromViewNode', () => {
  function makeWorkflowViewNode(
    overrides: Partial<WorkflowViewNode>,
  ): WorkflowViewNode {
    return {
      nodeId: 'wf-view-1',
      sessionId: 'session-1',
      status: 'pending',
      attempts: 0,
      workflowId: 'my-workflow',
      ...overrides,
    };
  }

  it('carries agentConfig.mode from view node', () => {
    const viewNode = makeWorkflowViewNode({ agentConfig: { mode: 'prompt' } });
    const result = toPipelineNodeFromViewNode(viewNode as ViewNode) as Extract<PipelineNode, { type?: 'workflow' }>;
    expect(result.agentConfig?.mode).toBe('prompt');
  });

  it('carries agentConfig.thoughtLevel from view node', () => {
    const viewNode = makeWorkflowViewNode({ agentConfig: { thoughtLevel: 'high' } });
    const result = toPipelineNodeFromViewNode(viewNode as ViewNode) as Extract<PipelineNode, { type?: 'workflow' }>;
    expect(result.agentConfig?.thoughtLevel).toBe('high');
  });

  it('carries agentConfig.model from view node', () => {
    const viewNode = makeWorkflowViewNode({ agentConfig: { model: 'claude-sonnet' } });
    const result = toPipelineNodeFromViewNode(viewNode as ViewNode) as Extract<PipelineNode, { type?: 'workflow' }>;
    expect(result.agentConfig?.model).toBe('claude-sonnet');
  });

  it('merges flat model with agentConfig, preferring agentConfig.model', () => {
    const viewNode = makeWorkflowViewNode({
      model: 'flat-model',
      agentConfig: { model: 'config-model', mode: 'workflow' },
    });
    const result = toPipelineNodeFromViewNode(viewNode as ViewNode) as Extract<PipelineNode, { type?: 'workflow' }>;
    expect(result.agentConfig?.model).toBe('config-model');
    expect(result.agentConfig?.mode).toBe('workflow');
  });

  it('falls back to flat model when agentConfig.model is absent', () => {
    const viewNode = makeWorkflowViewNode({
      model: 'flat-only-model',
      agentConfig: { mode: 'workflow', thoughtLevel: 'low' },
    });
    const result = toPipelineNodeFromViewNode(viewNode as ViewNode) as Extract<PipelineNode, { type?: 'workflow' }>;
    expect(result.agentConfig?.model).toBe('flat-only-model');
    expect(result.agentConfig?.mode).toBe('workflow');
    expect(result.agentConfig?.thoughtLevel).toBe('low');
  });

  it('carries configOptions through view node conversion', () => {
    const viewNode = makeWorkflowViewNode({
      agentConfig: { configOptions: { parallel: true } },
    });
    const result = toPipelineNodeFromViewNode(viewNode as ViewNode) as Extract<PipelineNode, { type?: 'workflow' }>;
    expect(result.agentConfig?.configOptions).toEqual({ parallel: true });
  });

  it('produces no agentConfig for a bare workflow view node', () => {
    const viewNode = makeWorkflowViewNode({});
    const result = toPipelineNodeFromViewNode(viewNode as ViewNode) as Extract<PipelineNode, { type?: 'workflow' }>;
    expect(result.agentConfig).toBeUndefined();
  });

  it('does not affect prompt view node conversion (no regression)', () => {
    const viewNode: ViewNode = {
      nodeId: 'prompt-view-1',
      sessionId: 'session-1',
      status: 'pending',
      attempts: 0,
      type: 'prompt',
      prompt: 'do work',
      agent: 'engineer',
      model: 'gpt-4',
    };
    const result = toPipelineNodeFromViewNode(viewNode) as Extract<PipelineNode, { type: 'prompt' }>;
    expect(result.model).toBe('gpt-4');
    expect(result.prompt).toBe('do work');
    expect(result.agent).toBe('engineer');
  });
});
