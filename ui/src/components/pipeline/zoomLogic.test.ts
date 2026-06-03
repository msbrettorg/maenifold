import { describe, expect, it } from 'vitest';
import {
  availableSurfaces,
  nodeConfigEntries,
  parseAssembledPrompt,
  prettyJson,
  sniffContentFormat,
  tryParseJsonStructure,
} from './zoomLogic';
import type {
  SequentialThinkingViewNode,
  ToolViewNode,
  WorkflowViewNode,
  PromptViewNode,
} from '../../lib/pipeline-types';

describe('sniffContentFormat', () => {
  it('detects a JSON object', () => {
    expect(sniffContentFormat('{"success":true,"data":"# Memory"}')).toBe('json');
  });

  it('detects a JSON array', () => {
    expect(sniffContentFormat('[1, 2, 3]')).toBe('json');
  });

  it('does NOT treat bare scalars as structured JSON', () => {
    expect(sniffContentFormat('42')).toBe('text');
    expect(sniffContentFormat('"just a quoted string"')).toBe('text');
  });

  it('falls back to text for malformed JSON-looking content', () => {
    expect(sniffContentFormat('{not valid json')).toBe('text');
  });

  it('detects markdown headings', () => {
    expect(sniffContentFormat('# Title\n\nsome body text')).toBe('markdown');
  });

  it('detects markdown lists and links', () => {
    expect(sniffContentFormat('- one\n- two')).toBe('markdown');
    expect(sniffContentFormat('see [docs](https://example.com) here')).toBe('markdown');
  });

  it('detects fenced code blocks', () => {
    expect(sniffContentFormat('```\ncode\n```')).toBe('markdown');
  });

  it('treats plain prose as text', () => {
    expect(sniffContentFormat('the quick brown fox jumped over the lazy dog')).toBe('text');
  });

  it('treats empty / whitespace as text', () => {
    expect(sniffContentFormat('')).toBe('text');
    expect(sniffContentFormat('   \n  ')).toBe('text');
  });

  it('prefers JSON over markdown when content is a JSON object even if it embeds markdown', () => {
    expect(sniffContentFormat('{"data":"# heading\\n- item"}')).toBe('json');
  });
});

describe('tryParseJsonStructure / prettyJson', () => {
  it('parses and pretty-prints an object', () => {
    const result = tryParseJsonStructure('{"a":1}');
    expect(result.ok).toBe(true);
    expect(prettyJson(result.value)).toBe('{\n  "a": 1\n}');
  });

  it('rejects scalars', () => {
    expect(tryParseJsonStructure('"x"').ok).toBe(false);
    expect(tryParseJsonStructure('7').ok).toBe(false);
  });
});

describe('parseAssembledPrompt', () => {
  it('returns parsed:false and whole text when no headers present', () => {
    const text = 'just one big prompt with no section markers at all';
    const result = parseAssembledPrompt(text);
    expect(result.parsed).toBe(false);
    expect(result.whole).toBe(text);
    expect(result.predecessor).toBeUndefined();
    expect(result.ambient).toBeUndefined();
    expect(result.instruction).toBeUndefined();
  });

  it('splits markdown-heading sections into the three buckets', () => {
    const text = [
      '# Predecessor Output',
      'result from node n1',
      '# Ambient Context',
      'memory and background',
      '# Instruction',
      'do the task now',
    ].join('\n');
    const result = parseAssembledPrompt(text);
    expect(result.parsed).toBe(true);
    expect(result.predecessor).toBe('result from node n1');
    expect(result.ambient).toBe('memory and background');
    expect(result.instruction).toBe('do the task now');
  });

  it('handles Label: style headers', () => {
    const text = ['Upstream:', 'from above', 'Task:', 'execute'].join('\n');
    const result = parseAssembledPrompt(text);
    expect(result.parsed).toBe(true);
    expect(result.predecessor).toBe('from above');
    expect(result.instruction).toBe('execute');
  });

  it('concatenates multiple sections that map to the same bucket', () => {
    const text = ['# Context', 'first', '# Background', 'second'].join('\n');
    const result = parseAssembledPrompt(text);
    expect(result.ambient).toBe('first\n\nsecond');
  });

  it('ignores headers with empty bodies', () => {
    const text = ['# Instruction', '', '# Context', 'real body'].join('\n');
    const result = parseAssembledPrompt(text);
    expect(result.instruction).toBeUndefined();
    expect(result.ambient).toBe('real body');
  });

  it('always exposes the whole original text', () => {
    const text = '# Task\nbody';
    expect(parseAssembledPrompt(text).whole).toBe(text);
  });
});

function baseFields(nodeId: string) {
  return { nodeId, sessionId: `s-${nodeId}`, status: 'pending' as const, attempts: 0 };
}

describe('availableSurfaces', () => {
  it('prompt node with assembledPrompt exposes prompt', () => {
    const node: PromptViewNode = {
      ...baseFields('p1'),
      type: 'prompt',
      prompt: 'hi',
      agent: 'copilot',
      assembledPrompt: 'assembled text',
    };
    expect(availableSurfaces(node)).toEqual(['prompt']);
  });

  it('sequentialThinking node exposes prompt + thinking + output together', () => {
    const node: SequentialThinkingViewNode = {
      ...baseFields('t1'),
      type: 'sequentialThinking',
      topic: 'analyze',
      agent: 'copilot',
      assembledPrompt: 'assembled',
      thinkingTrace: [{ thoughtNumber: 1, text: 'first thought' }],
      output: 'some output',
    };
    expect(availableSurfaces(node)).toEqual(['prompt', 'thinking', 'output']);
  });

  it('workflow node with steps exposes workflow', () => {
    const node: WorkflowViewNode = {
      ...baseFields('w1'),
      type: 'workflow',
      workflowId: 'abductive',
      workflowSteps: [{ stepName: 'step', status: 'done' }],
    };
    expect(availableSurfaces(node)).toEqual(['workflow']);
  });

  it('tool node with output + provenance exposes output then provenance', () => {
    const node: ToolViewNode = {
      ...baseFields('tool1'),
      type: 'tool',
      toolName: 'MemoryStatus',
      output: '{"success":true}',
      toolProvenance: { toolName: 'MemoryStatus', resolvedArgs: {}, responseSummary: 'ok' },
    };
    expect(availableSurfaces(node)).toEqual(['output', 'provenance']);
  });

  it('returns empty list for a node with no surfaces', () => {
    const node: PromptViewNode = {
      ...baseFields('empty'),
      type: 'prompt',
      prompt: 'hi',
      agent: 'copilot',
    };
    expect(availableSurfaces(node)).toEqual([]);
  });

  it('ignores empty-string assembledPrompt and output', () => {
    const node: PromptViewNode = {
      ...baseFields('blank'),
      type: 'prompt',
      prompt: 'hi',
      agent: 'copilot',
      assembledPrompt: '   ',
      output: '',
    };
    expect(availableSurfaces(node)).toEqual([]);
  });

  it('does not expose thinking for an empty trace array', () => {
    const node: SequentialThinkingViewNode = {
      ...baseFields('t2'),
      type: 'sequentialThinking',
      topic: 'analyze',
      agent: 'copilot',
      thinkingTrace: [],
    };
    expect(availableSurfaces(node)).toEqual([]);
  });
});

describe('nodeConfigEntries', () => {
  it('returns no entries for a node with no config fields', () => {
    const node: PromptViewNode = {
      ...baseFields('p0'),
      type: 'prompt',
      prompt: 'hi',
      agent: 'copilot',
    };
    expect(nodeConfigEntries(node)).toEqual([]);
  });

  it('pretty-prints agentConfig and schemas as JSON blocks', () => {
    const node: PromptViewNode = {
      ...baseFields('p1'),
      type: 'prompt',
      prompt: 'hi',
      agent: 'copilot',
      agentConfig: { model: 'gpt-5.4', mode: 'fast' },
      inputSchema: { type: 'object' },
    };
    const entries = nodeConfigEntries(node);
    const agentCfg = entries.find((e) => e.label === 'agentConfig');
    expect(agentCfg?.block).toBe(true);
    expect(agentCfg?.value).toBe('{\n  "model": "gpt-5.4",\n  "mode": "fast"\n}');
    expect(entries.find((e) => e.label === 'inputSchema')?.block).toBe(true);
  });

  it('pretty-prints tool toolArgs and renders scalars/list inline', () => {
    const node: ToolViewNode = {
      ...baseFields('tool1'),
      type: 'tool',
      toolName: 'RecentActivity',
      toolArgs: { limit: 3 },
      cwd: 'C:/src',
      timeout: 5000,
      allowedTools: ['a', 'b'],
      maxTurns: 4,
    };
    const entries = nodeConfigEntries(node);
    expect(entries.find((e) => e.label === 'toolArgs')?.block).toBe(true);
    expect(entries.find((e) => e.label === 'cwd')).toEqual({
      label: 'cwd',
      value: 'C:/src',
      block: false,
    });
    expect(entries.find((e) => e.label === 'timeout')?.value).toBe('5000');
    expect(entries.find((e) => e.label === 'allowedTools')?.value).toBe('a, b');
    expect(entries.find((e) => e.label === 'maxTurns')?.value).toBe('4');
  });

  it('omits null/empty config fields (no blank rows)', () => {
    const node: ToolViewNode = {
      ...baseFields('tool2'),
      type: 'tool',
      toolName: 'MemoryStatus',
      toolArgs: {}, // empty object → omitted
      cwd: null,
      timeout: null,
      allowedTools: [],
      agentConfig: null,
    };
    expect(nodeConfigEntries(node)).toEqual([]);
  });

  it('previews sequentialThinking config (totalThoughts/analysisType)', () => {
    const node: SequentialThinkingViewNode = {
      ...baseFields('t1'),
      type: 'sequentialThinking',
      topic: 'analyze',
      agent: 'copilot',
      totalThoughts: 5,
      analysisType: 'critical',
      thoughtLevel: 'deep',
    };
    const entries = nodeConfigEntries(node);
    expect(entries.find((e) => e.label === 'totalThoughts')?.value).toBe('5');
    expect(entries.find((e) => e.label === 'analysisType')?.value).toBe('critical');
    expect(entries.find((e) => e.label === 'thoughtLevel')?.value).toBe('deep');
  });

  // Blue-team (T-600.6 UAT): the live uat-canvas-demo prompt nodes render
  // `toolMode: disabled` as an inline scalar config entry; lock that path.
  it('renders toolMode as an inline scalar entry', () => {
    const node: PromptViewNode = {
      ...baseFields('p2'),
      type: 'prompt',
      prompt: 'hi',
      agent: 'copilot',
      toolMode: 'disabled',
    };
    expect(nodeConfigEntries(node)).toEqual([
      { label: 'toolMode', value: 'disabled', block: false },
    ]);
  });

  it('orders config entries deterministically (agentConfig → allowedTools → cwd)', () => {
    const node: ToolViewNode = {
      ...baseFields('tool3'),
      type: 'tool',
      toolName: 'RecentActivity',
      cwd: 'C:/src',
      allowedTools: ['x'],
      agentConfig: { k: 'v' },
    };
    expect(nodeConfigEntries(node).map((e) => e.label)).toEqual([
      'agentConfig',
      'allowedTools',
      'cwd',
    ]);
  });
});

// Additional pure-logic edge cases added during T-600.6 blue-team verification.
// These cover header/marker variants and priority that the existing suite did
// not exercise; they only ADD coverage and assert against the real logic.
describe('zoomLogic blue-team coverage (T-600.6)', () => {
  it('sniff: detects blockquote, ordered list, and table-row markdown', () => {
    expect(sniffContentFormat('> a quoted line')).toBe('markdown');
    expect(sniffContentFormat('1. first\n2. second')).toBe('markdown');
    expect(sniffContentFormat('| col a | col b |')).toBe('markdown');
  });

  it('sniff: detects inline bold and inline code', () => {
    expect(sniffContentFormat('this is **bold** prose')).toBe('markdown');
    expect(sniffContentFormat('run `npm test` to verify')).toBe('markdown');
  });

  it('sniff: JSON wins even with leading whitespace before the brace', () => {
    expect(sniffContentFormat('   \n  {"a":1}')).toBe('json');
    expect(sniffContentFormat('  [1,2,3]')).toBe('json');
  });

  it('parseAssembledPrompt: classifies [Bracket] banner headers', () => {
    const text = ['[Incoming]', 'data from upstream', '[Goal]', 'do the task'].join('\n');
    const result = parseAssembledPrompt(text);
    expect(result.parsed).toBe(true);
    expect(result.predecessor).toBe('data from upstream');
    expect(result.instruction).toBe('do the task');
  });

  it('parseAssembledPrompt: classifies === banner === headers', () => {
    const text = ['=== Context ===', 'ambient background body'].join('\n');
    const result = parseAssembledPrompt(text);
    expect(result.parsed).toBe(true);
    expect(result.ambient).toBe('ambient background body');
  });

  it('parseAssembledPrompt: a header matching several families lands in the highest-priority bucket', () => {
    // "Input Instruction" matches both predecessor (input) and instruction
    // (instruction); KIND_ORDER puts predecessor first, so it wins.
    const result = parseAssembledPrompt(['# Input Instruction', 'body text'].join('\n'));
    expect(result.predecessor).toBe('body text');
    expect(result.instruction).toBeUndefined();
  });
});
