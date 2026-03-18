import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { MemoryFile, RecentResult, SearchResult } from '../../lib/workflow-types';
import { useWorkflowTree } from '../useWorkflowTree';

const apiMocks = vi.hoisted(() => ({
  readMemory: vi.fn<(uri: string) => Promise<MemoryFile>>(),
  searchMemories: vi.fn<(query: string, opts?: unknown) => Promise<SearchResult>>(),
  recentActivity: vi.fn<(opts?: unknown) => Promise<RecentResult>>(),
}));

vi.mock('../useMemoryApi', () => ({
  useMemoryApi: () => apiMocks,
}));

const rootUri = 'memory://thinking/workflow/2026/03/17/workflow-1773720937011';
const step1SessionId = 'session-1773720949468-20935';
const step2SessionId = 'session-1773721125868-41308';
const step3SessionId = 'session-1773721216626-22867';
const step4SessionId = 'session-1773721291888-29182';
const childWorkflowId = 'workflow-1773721328318';
const step3FallbackUri = `memory://thinking/sequential/2026/03/16/${step3SessionId}`;
const childWorkflowFallbackUri = `memory://thinking/workflow/2026/03/18/${childWorkflowId}`;

function memoryFile(uri: string, title: string, content: string, created: string): MemoryFile {
  return {
    uri,
    title,
    content,
    created,
    modified: created,
  };
}

function sequentialContent(totalThoughts: number, sessionLabel: string): string {
  const thoughts = Array.from({ length: totalThoughts }, (_value, index) => {
    const thoughtNumber = index + 1;
    const minute = String(10 + thoughtNumber).padStart(2, '0');
    return [
      `## Thought ${thoughtNumber}/${totalThoughts} [agent]`,
      `${sessionLabel} thought ${thoughtNumber} with [[traceability]].`,
      '',
      `*2026-03-17 10:${minute}:00 UTC*`,
    ].join('\n');
  });

  return [
    '# Sequential Thinking Session',
    '',
    ...thoughts,
    '',
    '## Conclusion',
    '',
    `${sessionLabel} complete with [[traceability]].`,
  ].join('\n');
}

describe('useWorkflowTree', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('parses workflow and sequential markdown into a recursive execution tree', async () => {
    const memories: Record<string, MemoryFile> = {
      [rootUri]: memoryFile(
        rootUri,
        'workflow-dispatch',
        [
          '# Workflow Session',
          '',
          'Started: 2026-03-17 10:00:00 UTC',
          '',
          '## Step 1/4',
          '',
          '{',
          '  "id": "gather-context",',
          '  "name": "Gather context",',
          '  "requiresSequentialThinking": true',
          '}',
          '',
          '## Step 1/4 Response',
          '',
          `Created session: ${step1SessionId}`,
          '',
          '*2026-03-17 10:02:00 UTC*',
          '',
          '## Submachine Registered',
          '',
          `Submachine registered: sequential ${step1SessionId}`,
          '',
          '*2026-03-17 10:02:01 UTC*',
          '',
          '## Step 2/4',
          '',
          '{',
          '  "id": "compare-options",',
          '  "name": "Compare options",',
          '  "requiresSequentialThinking": true',
          '}',
          '',
          '## Step 2/4 Response',
          '',
          `Created session: ${step2SessionId}`,
          '',
          '*2026-03-17 10:04:00 UTC*',
          '',
          '## Step 3/4',
          '',
          '{',
          '  "id": "evaluate-risks",',
          '  "name": "Evaluate risks",',
          '  "requiresSequentialThinking": true',
          '}',
          '',
          '## Step 3/4 Response',
          '',
          `Created session: ${step3SessionId}`,
          '',
          '*2026-03-17 10:06:00 UTC*',
          '',
          '## Step 4/4',
          '',
          '{',
          '  "id": "dispatch-child",',
          '  "name": "Dispatch child workflow",',
          '  "requiresSequentialThinking": true',
          '}',
          '',
          '## Step 4/4 Response',
          '',
          `Created session: ${step4SessionId}`,
          `Dispatched child workflow ${childWorkflowId} for critical-thinking follow-up.`,
          '',
          '*2026-03-17 10:08:00 UTC*',
          '',
          '## Conclusion',
          '',
          'Workflow dispatch complete with [[traceability]].',
        ].join('\n'),
        '2026-03-17T10:00:00Z',
      ),
      [`memory://thinking/sequential/2026/03/17/${step1SessionId}`]: memoryFile(
        `memory://thinking/sequential/2026/03/17/${step1SessionId}`,
        step1SessionId,
        sequentialContent(5, 'Step 1'),
        '2026-03-17T10:02:00Z',
      ),
      [`memory://thinking/sequential/2026/03/17/${step2SessionId}`]: memoryFile(
        `memory://thinking/sequential/2026/03/17/${step2SessionId}`,
        step2SessionId,
        sequentialContent(4, 'Step 2'),
        '2026-03-17T10:04:00Z',
      ),
      [step3FallbackUri]: memoryFile(
        step3FallbackUri,
        step3SessionId,
        sequentialContent(3, 'Step 3'),
        '2026-03-16T23:59:00Z',
      ),
      [`memory://thinking/sequential/2026/03/17/${step4SessionId}`]: memoryFile(
        `memory://thinking/sequential/2026/03/17/${step4SessionId}`,
        step4SessionId,
        sequentialContent(3, 'Step 4'),
        '2026-03-17T10:08:00Z',
      ),
      [childWorkflowFallbackUri]: memoryFile(
        childWorkflowFallbackUri,
        'critical-thinking',
        [
          '# Workflow Session',
          '',
          'Started: 2026-03-18 09:00:00 UTC',
          '',
          '## Step 1/7',
          '',
          '{',
          '  "id": "frame-problem",',
          '  "name": "Frame the problem",',
          '  "requiresSequentialThinking": false',
          '}',
          '',
          '## Step 1/7 Response',
          '',
          'Captured framing notes.',
          '',
          '*2026-03-18 09:02:00 UTC*',
          '',
          '## Step 2/7',
          '',
          '{',
          '  "id": "collect-inputs",',
          '  "name": "Collect inputs",',
          '  "requiresSequentialThinking": false',
          '}',
          '',
          '## Step 2/7 Response',
          '',
          'Collected supporting material.',
          '',
          '*2026-03-18 09:05:00 UTC*',
          '',
          '## Step 3/7',
          '',
          '{ "id": "stress-test", "name": "Stress test assumptions", "requiresSequentialThinking": false }',
          '',
          '## Step 4/7',
          '',
          '{ "id": "branch-scenarios", "name": "Branch scenarios", "requiresSequentialThinking": false }',
          '',
          '## Step 5/7',
          '',
          '{ "id": "rate-options", "name": "Rate options", "requiresSequentialThinking": false }',
          '',
          '## Step 6/7',
          '',
          '{ "id": "draft", "name": "Draft recommendation", "requiresSequentialThinking": false }',
          '',
          '## Step 7/7',
          '',
          '{ "id": "handoff", "name": "Prepare handoff", "requiresSequentialThinking": false }',
          '',
          '## Session Abandoned',
          '',
          '⚠️ Session marked as abandoned due to inactivity.',
          '',
          '*2026-03-18 09:10:00 UTC*',
        ].join('\n'),
        '2026-03-18T09:00:00Z',
      ),
    };

    apiMocks.readMemory.mockImplementation(async (uri: string) => {
      const memory = memories[uri];
      if (!memory) {
        throw new Error(`missing memory: ${uri}`);
      }
      return memory;
    });

    apiMocks.searchMemories.mockImplementation(async (query: string) => {
      if (query === step3SessionId) {
        return {
          results: [
            {
              uri: step3FallbackUri,
              title: step3SessionId,
              snippet: step3SessionId,
              score: 1,
            },
          ],
          totalCount: 1,
          page: 1,
          pageSize: 25,
        };
      }

      return {
        results: [],
        totalCount: 0,
        page: 1,
        pageSize: 25,
      };
    });

    apiMocks.recentActivity.mockResolvedValue({
      items: [
        {
          uri: childWorkflowFallbackUri,
          title: childWorkflowId,
          modified: '2026-03-18T09:10:00Z',
          sections: ['Session Abandoned'],
        },
      ],
    });

    const { loadTree, tree, loading, error } = useWorkflowTree();
    const result = await loadTree(rootUri);

    expect(loading.value).toBe(false);
    expect(error.value).toBeNull();
    expect(tree.value).toEqual(result);

    expect(result.type).toBe('workflow');
    if (result.type !== 'workflow') {
      throw new Error('expected workflow root');
    }

    expect(result.data.workflowId).toBe('workflow-dispatch');
    expect(result.data.workflowName).toBe('workflow-dispatch');
    expect(result.data.status).toBe('completed');
    expect(result.data.started).toBe('2026-03-17T10:00:00Z');
    expect(result.data.steps).toHaveLength(4);
    expect(result.data.steps.map((step) => step.submachineSessionId)).toEqual([
      step1SessionId,
      step2SessionId,
      step3SessionId,
      step4SessionId,
    ]);
    expect(result.data.steps[0]).toMatchObject({
      id: 'gather-context',
      name: 'Gather context',
      status: 'completed',
      respondedAt: '2026-03-17T10:02:00Z',
      requiresSequentialThinking: true,
    });
    expect(result.data.steps[3].response).toContain(childWorkflowId);
    expect(result.data.childWorkflowUri).toBe(childWorkflowFallbackUri);

    expect(result.children).toHaveLength(5);
    expect(result.children.slice(0, 4).every((node) => node.type === 'thinking')).toBe(true);
    expect(result.children[4]?.type).toBe('workflow');

    const [step1Node, step2Node, step3Node, step4Node, childNode] = result.children;
    for (const [node, count] of [[step1Node, 5], [step2Node, 4], [step3Node, 3], [step4Node, 3]] as const) {
      expect(node.type).toBe('thinking');
      if (node.type !== 'thinking') {
        continue;
      }
      expect(node.data.status).toBe('completed');
      expect(node.data.thoughts).toHaveLength(count);
      expect(node.data.thoughts[0]?.content).not.toContain('UTC');
    }

    expect(childNode.type).toBe('workflow');
    if (childNode.type !== 'workflow') {
      throw new Error('expected child workflow');
    }
    expect(childNode.data.workflowId).toBe('critical-thinking');
    expect(childNode.data.workflowName).toBe('critical-thinking');
    expect(childNode.data.status).toBe('abandoned');
    expect(childNode.data.steps).toHaveLength(7);
    expect(childNode.data.steps[0]?.status).toBe('completed');
    expect(childNode.data.steps[2]?.status).toBe('pending');

    expect(apiMocks.searchMemories).toHaveBeenCalledWith(step3SessionId, expect.objectContaining({
      folder: 'thinking/sequential',
      pageSize: 25,
    }));
    expect(apiMocks.searchMemories).toHaveBeenCalledWith(childWorkflowId, expect.objectContaining({
      folder: 'thinking/workflow',
      pageSize: 25,
    }));
    expect(apiMocks.recentActivity).toHaveBeenCalledWith(expect.objectContaining({
      filter: 'thinking',
      limit: 200,
    }));
  });
});
