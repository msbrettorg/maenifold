import { describe, expect, it } from 'vitest';
import {
  INSPECTION_GRACE_MS,
  graceRemainingMs,
  hiddenInferenceErrors,
  inspectionGuard,
  isInspectableNode,
  isPastGrace,
  resolveNodeAgent,
  resolveNodeModel,
  resolveNodeSessionId,
} from './sessionResolve';
import type {
  PromptViewNode,
  SequentialThinkingViewNode,
  ToolViewNode,
  ViewNode,
  WorkflowViewNode,
} from './pipeline-types';

// Blue-team coverage (T-600.7): sessionResolve is the pure guard/resolution
// layer that decides whether a running node is inspectable. Its correctness is
// constitutional — a wrong "ok" here would let maenifold reason through a session
// the operator cannot see (hidden inference), the exact failure this task exists
// to prevent.

function promptNode(over: Partial<PromptViewNode> = {}): PromptViewNode {
  return {
    type: 'prompt',
    nodeId: 'n-prompt',
    sessionId: 'pipeline-sess',
    status: 'active',
    attempts: 1,
    prompt: 'do the thing',
    agent: 'copilot',
    model: 'gpt-5',
    ...over,
  };
}

function workflowNode(over: Partial<WorkflowViewNode> = {}): WorkflowViewNode {
  return {
    type: 'workflow',
    nodeId: 'n-wf',
    sessionId: 'pipeline-sess',
    status: 'active',
    attempts: 1,
    workflowId: 'wf-1',
    ...over,
  };
}

function toolNode(over: Partial<ToolViewNode> = {}): ToolViewNode {
  return {
    type: 'tool',
    nodeId: 'n-tool',
    sessionId: 'pipeline-sess',
    status: 'active',
    attempts: 1,
    toolName: 'maenifold.memory/ReadMemory',
    ...over,
  };
}

describe('isInspectableNode', () => {
  it('treats tool nodes as non-inspectable and reasoning nodes as inspectable', () => {
    expect(isInspectableNode(toolNode())).toBe(false);
    expect(isInspectableNode(promptNode())).toBe(true);
    expect(isInspectableNode(workflowNode())).toBe(true);
  });
});

describe('resolveNodeSessionId — priority order', () => {
  it('prefers provenance.detail.sessionId above all', () => {
    const node = promptNode({
      acpSessionId: 'acp-2',
      sessionRef: { agent: 'copilot', sessionId: 'ref-3' },
      provenance: { inspectable: true, agent: 'copilot', detail: { sessionId: 'prov-1' } },
    });
    expect(resolveNodeSessionId(node)).toBe('prov-1');
  });

  it('falls back to acpSessionId, then sessionRef.sessionId', () => {
    expect(resolveNodeSessionId(promptNode({ acpSessionId: 'acp-2', sessionRef: { agent: 'copilot', sessionId: 'ref-3' } }))).toBe(
      'acp-2'
    );
    expect(resolveNodeSessionId(promptNode({ sessionRef: { agent: 'copilot', sessionId: 'ref-3' } }))).toBe('ref-3');
  });

  it('returns null when no acp session id is present (the pipeline sessionId is NOT used)', () => {
    expect(resolveNodeSessionId(promptNode())).toBeNull();
  });

  it('ignores blank/whitespace ids', () => {
    expect(resolveNodeSessionId(promptNode({ acpSessionId: '   ' }))).toBeNull();
  });
});

describe('resolveNodeAgent / resolveNodeModel', () => {
  it('reads agent from the node, then acpClient, then sessionRef, then provenance', () => {
    expect(resolveNodeAgent(promptNode({ agent: 'a1' }))).toBe('a1');
    expect(resolveNodeAgent(workflowNode({ acpClient: 'a2' }))).toBe('a2');
    expect(
      resolveNodeAgent(promptNode({ agent: '', sessionRef: { agent: 'a3', sessionId: 'x' } }) as ViewNode)
    ).toBe('a3');
    expect(
      resolveNodeAgent(
        workflowNode({ provenance: { inspectable: true, agent: 'a4', detail: {} } })
      )
    ).toBe('a4');
  });

  it('reads model from the explicit field, then agentConfig.model', () => {
    expect(resolveNodeModel(promptNode({ model: 'm1' }))).toBe('m1');
    expect(resolveNodeModel(promptNode({ model: null, agentConfig: { model: 'm2' } }))).toBe('m2');
    expect(resolveNodeModel(promptNode({ model: null }))).toBeNull();
  });
});

describe('inspectionGuard', () => {
  it('is ok only when sessionId, agent and model are all present', () => {
    const node = promptNode({ acpSessionId: 'acp-1' });
    const guard = inspectionGuard(node);
    expect(guard.ok).toBe(true);
    expect(guard.missing).toEqual([]);
    expect(guard).toMatchObject({ sessionId: 'acp-1', agent: 'copilot', model: 'gpt-5' });
  });

  it('names every missing field in display order', () => {
    const guard = inspectionGuard(promptNode({ agent: '', model: null }) as ViewNode);
    expect(guard.ok).toBe(false);
    expect(guard.missing).toEqual(['sessionId', 'agent', 'model']);
  });
});

describe('grace window', () => {
  it('isPastGrace is false within the window and true after it', () => {
    const start = 1_000_000;
    expect(isPastGrace(start, start)).toBe(false);
    expect(isPastGrace(start, start + INSPECTION_GRACE_MS - 1)).toBe(false);
    expect(isPastGrace(start, start + INSPECTION_GRACE_MS)).toBe(true);
  });

  it('treats an unknown graceStart as past grace (no warm-up basis)', () => {
    expect(isPastGrace(undefined, 5)).toBe(true);
  });

  it('graceRemainingMs counts down to zero', () => {
    const start = 2_000_000;
    expect(graceRemainingMs(start, start)).toBe(INSPECTION_GRACE_MS);
    expect(graceRemainingMs(start, start + 10_000)).toBe(INSPECTION_GRACE_MS - 10_000);
    expect(graceRemainingMs(start, start + INSPECTION_GRACE_MS + 5)).toBe(0);
    expect(graceRemainingMs(undefined, 5)).toBe(0);
  });
});

describe('hiddenInferenceErrors', () => {
  const seqNode: SequentialThinkingViewNode = {
    type: 'sequentialThinking',
    nodeId: 'n-think',
    sessionId: 'pipeline-sess',
    status: 'active',
    attempts: 1,
    topic: 'reasoning',
    agent: 'copilot',
    displayLabel: 'Think hard',
    // no sessionId / model resolvable → guard fails
  };

  it('flags an active inspectable node that fails the guard past grace', () => {
    const now = 5_000_000;
    const grace = new Map([['n-think', now - INSPECTION_GRACE_MS]]);
    const errors = hiddenInferenceErrors([seqNode], new Set(['n-think']), grace, now);
    expect(errors).toHaveLength(1);
    expect(errors[0]).toMatchObject({
      nodeId: 'n-think',
      displayLabel: 'Think hard',
      missing: ['sessionId', 'model'],
    });
  });

  it('does NOT flag a node still inside its grace window', () => {
    const now = 5_000_000;
    const grace = new Map([['n-think', now - 1_000]]);
    expect(hiddenInferenceErrors([seqNode], new Set(['n-think']), grace, now)).toEqual([]);
  });

  it('does NOT flag inactive nodes, tool nodes, or guard-ok nodes', () => {
    const now = 5_000_000;
    const grace = new Map([
      ['n-think', now - INSPECTION_GRACE_MS],
      ['n-tool', now - INSPECTION_GRACE_MS],
      ['n-prompt', now - INSPECTION_GRACE_MS],
    ]);
    const tool = toolNode({ nodeId: 'n-tool' });
    const okPrompt = promptNode({ nodeId: 'n-prompt', acpSessionId: 'acp-1' });
    // n-think present but NOT in the active set → ignored.
    const errors = hiddenInferenceErrors(
      [seqNode, tool, okPrompt],
      new Set(['n-tool', 'n-prompt']),
      grace,
      now
    );
    expect(errors).toEqual([]);
  });
});
