// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ref } from 'vue';
import { useMemoryApi } from '../useMemoryApi';

const mockGatewayExecute = vi.fn();

vi.mock('../useSharedMcpClient', () => ({
  getSharedMcpClient: vi.fn(() => ({
    connected: ref(true),
    connect: vi.fn(),
    error: ref(null),
    onNotification: vi.fn(() => vi.fn()),
    onDisconnect: vi.fn(() => vi.fn()),
    gatewayExecute: mockGatewayExecute,
  })),
}));

describe('useMemoryApi', () => {
  beforeEach(() => {
    mockGatewayExecute.mockReset();
  });

  it('calls ReadMemory with the canonical gateway envelope', async () => {
    mockGatewayExecute.mockResolvedValue({
      uri: 'memory://research/example',
      content: '# Example',
    });

    await useMemoryApi().readMemory('memory://research/example');

    expect(mockGatewayExecute).toHaveBeenCalledWith(
      'maenifold.memory',
      'ReadMemory',
      { identifier: 'memory://research/example' },
    );
  });

  it('calls SearchMemories with the canonical gateway envelope', async () => {
    mockGatewayExecute.mockResolvedValue({
      results: [{ uri: 'memory://research/example', title: 'Example', snippet: 'snippet', score: 0.9 }],
      totalCount: 1,
      page: 2,
      pageSize: 5,
    });

    await useMemoryApi().searchMemories('example', {
      mode: 'Hybrid',
      pageSize: 5,
      page: 2,
      folder: 'research',
    });

    expect(mockGatewayExecute).toHaveBeenCalledWith(
      'maenifold.memory',
      'SearchMemories',
      {
        query: 'example',
        mode: 'Hybrid',
        pageSize: 5,
        page: 2,
        folder: 'research',
      },
    );
  });

  it('calls RecentActivity with the canonical gateway envelope', async () => {
    mockGatewayExecute.mockResolvedValue({
      items: [{
        uri: 'memory://research/example',
        title: 'Example',
        modified: '2026-05-23T00:00:00Z',
        sections: ['Intro'],
      }],
    });

    await useMemoryApi().recentActivity({
      filter: 'all',
      limit: 7,
      timespan: '7d',
      includeContent: true,
    });

    expect(mockGatewayExecute).toHaveBeenCalledWith(
      'maenifold.system',
      'RecentActivity',
      {
        filter: 'all',
        limit: 7,
        timespan: '7d',
        includeContent: true,
      },
    );
  });

  it('calls ListMemories with the canonical gateway envelope', async () => {
    mockGatewayExecute.mockResolvedValue('memory://research/example');

    await useMemoryApi().listMemories({ path: 'research' });

    expect(mockGatewayExecute).toHaveBeenCalledWith(
      'maenifold.memory',
      'ListMemories',
      { path: 'research' },
    );
  });

  it('calls WriteMemory with the canonical gateway envelope', async () => {
    mockGatewayExecute.mockResolvedValue({ uri: 'memory://research/example' });

    await useMemoryApi().writeMemory({
      title: 'Example',
      folder: ' research ',
      content: '# Example',
    });

    expect(mockGatewayExecute).toHaveBeenCalledWith(
      'maenifold.memory',
      'WriteMemory',
      {
        title: 'Example',
        folder: 'research',
        content: '# Example',
      },
    );
  });

  it('calls EditMemory with the canonical gateway envelope', async () => {
    mockGatewayExecute.mockResolvedValue({ uri: 'memory://research/example' });

    await useMemoryApi().editMemory({
      identifier: 'memory://research/example',
      content: 'updated body',
      checksum: 'abc123',
      findText: 'old body',
    });

    expect(mockGatewayExecute).toHaveBeenCalledWith(
      'maenifold.memory',
      'EditMemory',
      {
        identifier: 'memory://research/example',
        operation: 'find_replace',
        findText: 'old body',
        expectedCount: 1,
        content: 'updated body',
        checksum: 'abc123',
      },
    );
  });

  it('calls DeleteMemory with the canonical gateway envelope', async () => {
    mockGatewayExecute.mockResolvedValue(undefined);

    await useMemoryApi().deleteMemory('memory://research/example');

    expect(mockGatewayExecute).toHaveBeenCalledWith(
      'maenifold.memory',
      'DeleteMemory',
      {
        identifier: 'memory://research/example',
        confirm: true,
      },
    );
  });

  it('calls BuildContext with the canonical gateway envelope', async () => {
    mockGatewayExecute.mockResolvedValue({
      conceptName: 'Example Concept',
      depth: 2,
      directRelations: [],
      expandedRelations: [],
    });

    await useMemoryApi().buildContext('Example Concept', {
      depth: 2,
      maxEntities: 15,
      includeContent: true,
    });

    expect(mockGatewayExecute).toHaveBeenCalledWith(
      'maenifold.graph',
      'BuildContext',
      {
        conceptName: 'Example Concept',
        depth: 2,
        maxEntities: 15,
        includeContent: true,
      },
    );
  });

  it('calls FindSimilarConcepts with the canonical gateway envelope', async () => {
    mockGatewayExecute.mockResolvedValue({
      conceptName: 'Example Concept',
      totalCount: 1,
      results: [{ name: 'Related Concept', similarity: 0.91 }],
    });

    await useMemoryApi().findSimilarConcepts('Example Concept', 4);

    expect(mockGatewayExecute).toHaveBeenCalledWith(
      'maenifold.graph',
      'FindSimilarConcepts',
      {
        conceptName: 'Example Concept',
        maxResults: 4,
      },
    );
  });

  it('returns the destination URI from MoveMemory arrow responses', async () => {
    mockGatewayExecute.mockResolvedValue('Moved memory FILE: memory://research/old → memory://research/new');

    const result = await useMemoryApi().moveMemory({
      source: 'memory://research/old',
      destination: 'memory://research/new',
    });

    expect(result).toBe('memory://research/new');
  });

  it('falls back to the caller destination when MoveMemory omits an arrow URI', async () => {
    mockGatewayExecute.mockResolvedValue('Moved memory FILE');

    const result = await useMemoryApi().moveMemory({
      source: 'memory://research/old',
      destination: 'memory://research/new',
    });

    expect(result).toBe('memory://research/new');
  });
});
