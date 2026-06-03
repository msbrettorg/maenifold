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
  executeGatewayMock.mockImplementation(async (_domain: string, command: string) => {
    switch (command) {
      case 'SearchMemories':
        return {
          results: [
            {
              title: 'Count Note',
              path: 'memory://notes/count-note',
            },
          ],
          totalCount: 1,
        };
      case 'ReadMemory':
        return {
          title: 'Count Note',
          uri: 'memory://notes/count-note',
          content: '# Count Note\n\nRelation counts.',
          checksum: 'checksum-count',
        };
      default:
        throw new Error(`Unexpected command: ${command}`);
    }
  });

  executeGatewayTextMock.mockResolvedValue('### Folders\n### Files\n');
  readGatewayResourceMock.mockImplementation(async (uri: string) => {
    if (uri === 'graph://stats') {
      return {
        node_count: 20,
        edge_count: 40,
      };
    }

    if (uri.startsWith('graph://concept/')) {
      return {
        concept: 'count-note',
        totalCount: 1,
        neighbors: [
          {
            name: 'single-edge-neighbor',
            coOccurrenceCount: 1,
            weightedScore: 0.456,
            sourceFileCount: 1,
            files: ['memory://notes/single-edge-neighbor'],
          },
        ],
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

describe('KnowledgeView relation counts', () => {
  it('renders integer edge counts while preserving weighted-score precision', async () => {
    const wrapper = mountKnowledgeView();

    await settle();
    await wrapper.findAll('[role="tab"]')[1]!.trigger('click');
    await settle();

    const relationText = wrapper.get('.relation-row').text();
    expect(relationText).toContain('1 edge /');
    expect(relationText).not.toContain('1.000 edges');
    expect(relationText).toContain('0.456 weighted');
  });
});
