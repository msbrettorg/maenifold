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

function installResizeObserverShim() {
  const ResizeObserverMock = class {
    private readonly callback: ResizeObserverCallback;

    constructor(callback: ResizeObserverCallback) {
      this.callback = callback;
    }

    observe(target: Element) {
      this.callback(
        [
          {
            target,
            contentRect: {
              width: 480,
              height: 320,
            },
          } as ResizeObserverEntry,
        ],
        this as unknown as ResizeObserver
      );
    }

    disconnect() {}

    unobserve() {}
  };

  vi.stubGlobal('ResizeObserver', ResizeObserverMock);
}

function installBoundingRectShim() {
  vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockImplementation(function (this: HTMLElement) {
    if (this.classList.contains('graph-canvas')) {
      return {
        x: 0,
        y: 0,
        width: 480,
        height: 320,
        top: 0,
        left: 0,
        right: 480,
        bottom: 320,
        toJSON: () => null,
      } as DOMRect;
    }

    return {
      x: 0,
      y: 0,
      width: 0,
      height: 0,
      top: 0,
      left: 0,
      right: 0,
      bottom: 0,
      toJSON: () => null,
    } as DOMRect;
  });
}

function configureGatewayMocks() {
  executeGatewayMock.mockImplementation(async (_domain: string, command: string, parameters?: Record<string, unknown>) => {
    switch (command) {
      case 'SearchMemories': {
        const query = String(parameters?.query ?? '');
        if (query === 'multi result query') {
          return {
            results: [
              {
                title: 'Alpha Result',
                path: 'memory://notes/alpha-result',
                snippet: 'alpha snippet',
                fusedScore: 0.91,
              },
              {
                title: 'Beta Result',
                path: 'memory://notes/beta-result',
                snippet: 'beta snippet',
                fusedScore: 0.72,
              },
              {
                title: 'Gamma Result',
                path: 'memory://notes/gamma-result',
                snippet: 'gamma snippet',
                fusedScore: 0.63,
              },
            ],
            totalCount: 3,
          };
        }

        if (query === 'missing topic') {
          return {
            results: [],
            totalCount: 0,
          };
        }

        return {
          results: [
            {
              title: 'Bootstrap Note',
              path: 'memory://notes/bootstrap-note',
              snippet: 'bootstrap snippet',
              fusedScore: 0.55,
            },
          ],
          totalCount: 1,
        };
      }
      case 'ReadMemory': {
        const identifier = String(parameters?.identifier ?? '');
        return {
          title: identifier.split('/').pop()?.replace(/-/g, ' ') ?? 'Bootstrap Note',
          uri: identifier,
          content: `# ${identifier}\n\nContent for ${identifier}.`,
          checksum: `checksum:${identifier}`,
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
        node_count: 10,
        edge_count: 20,
      };
    }

    if (uri.startsWith('graph://concept/')) {
      return {
        concept: 'bootstrap-note',
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
  installResizeObserverShim();
  installBoundingRectShim();
  configureGatewayMocks();
});

afterEach(() => {
  while (mountedWrappers.length > 0) {
    mountedWrappers.pop()?.unmount();
  }
  document.body.innerHTML = '';
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe('KnowledgeView search results', () => {
  it('renders every ranked result row and opens the clicked result', async () => {
    const wrapper = mountKnowledgeView();

    await settle();

    await wrapper.get('input[type="search"]').setValue('multi result query');
    await wrapper.get('form.kb-search').trigger('submit');
    await settle();

    const rows = wrapper.findAll('.search-result-row');
    expect(rows).toHaveLength(3);

    await rows[1]!.trigger('click');
    await settle();

    expect(executeGatewayMock).toHaveBeenCalledWith('maenifold.memory', 'ReadMemory', {
      identifier: 'memory://notes/beta-result',
      includeChecksum: true,
    });
  });

  it('renders an explicit empty state after a search with no matches', async () => {
    const wrapper = mountKnowledgeView();

    await settle();

    await wrapper.get('input[type="search"]').setValue('missing topic');
    await wrapper.get('form.kb-search').trigger('submit');
    await settle();

    expect(wrapper.text()).toContain('No matches for "missing topic"');
  });

  it('preserves the graph workspace when submitting a search from the graph tab', async () => {
    const wrapper = mountKnowledgeView();

    await settle();
    await wrapper.findAll('[role="tab"]')[1]!.trigger('click');
    await settle();

    await wrapper.get('input[type="search"]').setValue('multi result query');
    await wrapper.get('form.kb-search').trigger('submit');
    await settle();

    const tabs = wrapper.findAll('[role="tab"]');
    expect(tabs[1]!.attributes('aria-selected')).toBe('true');
    expect(tabs[0]!.attributes('aria-selected')).toBe('false');
  });
});
