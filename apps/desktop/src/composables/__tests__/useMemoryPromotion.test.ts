import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ref } from 'vue';
import {
  buildPromotionPreview,
  hasWikiLinkConnection,
  normalizeWikiLinkInput,
  useMemoryPromotion,
} from '../useMemoryPromotion';

const mockConnect = vi.fn();
const mockCallTool = vi.fn();

vi.mock('../useSharedMcpClient', () => ({
  getSharedMcpClient: vi.fn(() => ({
    connected: ref(true),
    connect: mockConnect,
    error: ref(null),
    callTool: mockCallTool,
  })),
}));

describe('useMemoryPromotion', () => {
  beforeEach(() => {
    mockConnect.mockReset();
    mockCallTool.mockReset();
  });

  it('normalizes plain connection input into wiki links', () => {
    expect(normalizeWikiLinkInput('Semantic Zoom, Pipeline UI')).toEqual([
      '[[semantic-zoom]]',
      '[[pipeline-ui]]',
    ]);
    expect(hasWikiLinkConnection('Semantic Zoom')).toBe(true);
    expect(hasWikiLinkConnection('   ')).toBe(false);
  });

  it('builds previews for the promoted content section', () => {
    const preview = buildPromotionPreview({
      pipelineId: 'pipe-1',
      nodeId: 'node-1',
      output: 'hello world',
    }, 'Semantic Zoom', '2026-01-01T00:00:00.000Z');

    expect(preview).toContain('Connections: [[semantic-zoom]]');
    expect(preview).toContain('## Promoted Content');
    expect(preview).toContain('hello world');
    expect(preview).toContain('Source: Pipeline `pipe-1`');
  });

  it('writes new notes through WriteMemory with normalized wiki links', async () => {
    mockCallTool.mockResolvedValueOnce({
      success: true,
      data: {
        uri: 'memory://notes/example',
      },
    });

    const promotion = useMemoryPromotion();
    const uri = await promotion.writeMemory({
      pipelineId: 'pipe-1',
      nodeId: 'node-1',
      output: 'promoted output',
    }, {
      title: 'Example Note',
      wikiLinkInput: 'Semantic Zoom',
      folder: 'research',
      tags: ['ui'],
    });

    expect(uri).toBe('memory://notes/example');
    expect(mockCallTool).toHaveBeenCalledWith('WriteMemory', expect.objectContaining({
      title: 'Example Note',
      folder: 'research',
      tags: ['ui'],
      content: expect.stringContaining('Connections: [[semantic-zoom]]'),
    }));
  });

  it('appends promotion blocks through EditMemory append', async () => {
    mockCallTool.mockResolvedValueOnce({
      success: true,
      data: {
        uri: 'memory://notes/example',
      },
    });

    const promotion = useMemoryPromotion();
    await promotion.appendMemory({
      pipelineId: 'pipe-1',
      nodeId: 'node-1',
      output: 'next output',
    }, {
      identifier: 'memory://notes/example',
      wikiLinkInput: '[[Pipeline UI]]',
    });

    expect(mockCallTool).toHaveBeenCalledWith('EditMemory', expect.objectContaining({
      identifier: 'memory://notes/example',
      operation: 'append',
      content: expect.stringContaining('Connections: [[pipeline-ui]]'),
    }));
  });

  it('updates notes through EditMemory replace_section', async () => {
    mockCallTool.mockResolvedValueOnce({
      success: true,
      data: {
        uri: 'memory://notes/example',
      },
    });

    const promotion = useMemoryPromotion();
    await promotion.updateMemory({
      pipelineId: 'pipe-1',
      nodeId: 'node-1',
      output: 'new output',
    }, {
      identifier: 'memory://notes/example',
      wikiLinkInput: 'pipeline ui',
      checksum: 'checksum-1',
    });

    expect(mockCallTool).toHaveBeenCalledWith('EditMemory', expect.objectContaining({
      identifier: 'memory://notes/example',
      operation: 'replace_section',
      sectionName: 'Promoted Content',
      checksum: 'checksum-1',
      content: expect.stringContaining('Connections: [[pipeline-ui]]'),
    }));
  });
});
