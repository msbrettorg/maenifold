/* @vitest-environment jsdom */

import { flushPromises, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { nextTick } from 'vue';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const {
  executeGatewayMock,
  executeGatewayTextMock,
  readGatewayResourceMock,
} = vi.hoisted(() => ({
  executeGatewayMock: vi.fn(),
  executeGatewayTextMock: vi.fn(),
  readGatewayResourceMock: vi.fn(),
}));

vi.mock('../lib/mcp/gateway', () => ({
  executeGateway: executeGatewayMock,
  executeGatewayText: executeGatewayTextMock,
  readGatewayResource: readGatewayResourceMock,
}));

import KnowledgeView from './KnowledgeView.vue';

const mountedWrappers: Array<ReturnType<typeof mount>> = [];

function configureGatewayMocks() {
  executeGatewayMock.mockImplementation(async (_domain: string, command: string, parameters?: Record<string, unknown>) => {
    switch (command) {
      case 'SearchMemories': {
        const query = String(parameters?.query ?? '');
        if (query === 'linked-concept') {
          return {
            results: [
              {
                title: 'Linked Concept Note',
                path: 'memory://notes/linked-concept',
                snippet: 'linked concept snippet',
                fusedScore: 0.81,
              },
            ],
            totalCount: 1,
          };
        }

        if (query === 'missing-concept') {
          return {
            results: [
              {
                title: 'Missing Concept Placeholder',
                snippet: 'graph only fallback',
                fusedScore: 0.42,
              },
            ],
            totalCount: 1,
          };
        }

        return {
          results: [
            {
              title: 'Note A',
              path: 'memory://notes/note-a',
              snippet: 'note a snippet',
              fusedScore: 0.75,
            },
          ],
          totalCount: 1,
        };
      }
      case 'ReadMemory': {
        const identifier = String(parameters?.identifier ?? '');
        if (identifier === 'memory://notes/linked-concept') {
          return {
            title: 'Linked Concept Note',
            uri: 'memory://notes/linked-concept',
            content: '# [[Linked Concept]]\n\nReadable content for concept C.',
            checksum: 'checksum-linked',
          };
        }

        return {
          title: 'Note A',
          uri: 'memory://notes/note-a',
          content: '# Note A\n\nStart on note A with [[linked-concept]] and [[missing-concept]].',
          checksum: 'checksum-a',
        };
      }
      default:
        throw new Error(`Unexpected command: ${command}`);
    }
  });

  executeGatewayTextMock.mockResolvedValue('### Folders\n### Files\n');
  readGatewayResourceMock.mockImplementation(async (uri: string) => {
    if (uri === 'graph://stats') {
      return {
        node_count: 12,
        edge_count: 24,
      };
    }

    if (uri.includes('linked-concept')) {
      return {
        concept: 'linked-concept',
        totalCount: 0,
        neighbors: [],
      };
    }

    if (uri.includes('missing-concept')) {
      return {
        concept: 'missing-concept',
        totalCount: 0,
        neighbors: [],
      };
    }

    if (uri.startsWith('graph://concept/')) {
      return {
        concept: 'note-a',
        totalCount: 0,
        neighbors: [],
      };
    }

    throw new Error(`Unexpected resource: ${uri}`);
  });
}

async function settle() {
  await nextTick();
  await flushPromises();
  await nextTick();
  await flushPromises();
  await nextTick();
}

function mountKnowledgeView() {
  const pinia = createPinia();
  setActivePinia(pinia);

  const wrapper = mount(KnowledgeView, {
    global: {
      plugins: [pinia],
    },
  });
  mountedWrappers.push(wrapper);
  return wrapper;
}

beforeEach(() => {
  executeGatewayMock.mockReset();
  executeGatewayTextMock.mockReset();
  readGatewayResourceMock.mockReset();
  configureGatewayMocks();
});

afterEach(() => {
  while (mountedWrappers.length > 0) {
    mountedWrappers.pop()?.unmount();
  }
  document.body.innerHTML = '';
  vi.restoreAllMocks();
});

describe('KnowledgeView wikilink navigation', () => {
  it('opens readable linked-note content instead of leaving the previous note selected', async () => {
    const wrapper = mountKnowledgeView();

    await settle();

    expect(wrapper.get('.viewer-header h2').text()).toBe('Note A');

    await wrapper.get('.wikilink').trigger('click');
    await settle();

    expect(wrapper.get('.viewer-header h2').text()).toBe('Linked Concept Note');
    expect(wrapper.text()).toContain('Readable content for concept C.');
    expect(wrapper.text()).not.toContain('Start on note A with');
  });

  it('keeps the workspace on readable viewer content after a wikilink click', async () => {
    const wrapper = mountKnowledgeView();

    await settle();

    await wrapper.get('.wikilink').trigger('click');
    await settle();

    const tabs = wrapper.findAll('[role="tab"]');
    expect(tabs[0]!.attributes('aria-selected')).toBe('true');
    expect(tabs[1]!.attributes('aria-selected')).toBe('false');
    expect(wrapper.get('.viewer-header code').text()).toBe('memory://notes/linked-concept');
  });

  it('lands on graph context instead of showing stale viewer content when the linked concept has no readable note', async () => {
    const wrapper = mountKnowledgeView();

    await settle();

    await wrapper.findAll('.wikilink')[1]!.trigger('click');
    await settle();

    const tabs = wrapper.findAll('[role="tab"]');
    expect(tabs[0]!.attributes('aria-selected')).toBe('false');
    expect(tabs[1]!.attributes('aria-selected')).toBe('true');
    expect(wrapper.find('.viewer-header').exists()).toBe(false);
    expect(wrapper.get('.graph-meta').text()).toContain('missing-concept');
  });
});
