/* @vitest-environment jsdom */

import { flushPromises, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { nextTick } from 'vue';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const GRAPH_CAP = 28;
const TOTAL_NEIGHBORS = 73;
const GRAPH_CANVAS_WIDTH = 480;
const GRAPH_CANVAS_HEIGHT = 320;

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

function makeRelation(index: number) {
  return {
    name: `neighbor-${index + 1}`,
    coOccurrenceCount: 10 + index,
    weightedScore: 20 + index,
    sourceFileCount: 1,
    files: [`memory://knowledge/neighbor-${index + 1}`],
  };
}

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
              width: GRAPH_CANVAS_WIDTH,
              height: GRAPH_CANVAS_HEIGHT,
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
        width: GRAPH_CANVAS_WIDTH,
        height: GRAPH_CANVAS_HEIGHT,
        top: 0,
        left: 0,
        right: GRAPH_CANVAS_WIDTH,
        bottom: GRAPH_CANVAS_HEIGHT,
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
              title: 'Knowledge Graph Layout',
              path: 'memory://knowledge/knowledge-graph-layout',
            },
          ],
          totalCount: 1,
        };
      case 'ReadMemory':
        return {
          title: 'Knowledge Graph Layout',
          uri: 'memory://knowledge/knowledge-graph-layout',
          content: '# [[Knowledge Graph Layout]]\n\nGraph content.',
          checksum: 'checksum-1',
        };
      default:
        throw new Error(`Unexpected command: ${command}`);
    }
  });

  executeGatewayTextMock.mockResolvedValue('### Folders\n### Files\n');
  readGatewayResourceMock.mockImplementation(async (uri: string) => {
    if (uri === 'graph://stats') {
      return {
        node_count: 120,
        edge_count: 240,
      };
    }

    if (uri.startsWith('graph://concept/')) {
      return {
        concept: 'knowledge-graph-layout',
        totalCount: TOTAL_NEIGHBORS,
        neighbors: Array.from({ length: GRAPH_CAP }, (_, index) => makeRelation(index)),
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

async function openGraphTab(wrapper: ReturnType<typeof mount>) {
  await settle();
  await wrapper.findAll('[role="tab"]')[1]!.trigger('click');
  await settle();
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

describe('KnowledgeView', () => {
  it('renders one graph node per capped neighbor in the graph tab', async () => {
    const wrapper = mountKnowledgeView();

    await openGraphTab(wrapper);

    const renderedNeighbors = wrapper.findAll('.graph-node').filter((node) => !node.classes('focus'));
    expect(wrapper.get('.graph-view .pane-header strong').text()).toBe('73 neighbors');
    expect(renderedNeighbors).toHaveLength(GRAPH_CAP);
    expect(wrapper.findAll('.relation-row')).toHaveLength(GRAPH_CAP);
  });

  it('reports hidden neighbors from the number actually rendered in the graph', async () => {
    const wrapper = mountKnowledgeView();

    await openGraphTab(wrapper);

    const renderedNeighbors = wrapper.findAll('.graph-node').filter((node) => !node.classes('focus'));
    expect(wrapper.get('.graph-meta').text()).toContain(
      `${TOTAL_NEIGHBORS - renderedNeighbors.length} hidden`
    );
  });

  it('locks the graph stage to a square canvas and preserves SVG aspect ratio', async () => {
    const wrapper = mountKnowledgeView();

    await openGraphTab(wrapper);

    expect(wrapper.get('.graph-stage').attributes('style')).toContain(`width: ${GRAPH_CANVAS_HEIGHT}px;`);
    expect(wrapper.get('.graph-stage').attributes('style')).toContain(`height: ${GRAPH_CANVAS_HEIGHT}px;`);
    expect(wrapper.get('.graph-edges').attributes('preserveAspectRatio')).toBe('xMidYMid meet');
  });
});
