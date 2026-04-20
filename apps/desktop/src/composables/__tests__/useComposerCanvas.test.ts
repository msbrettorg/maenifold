// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, getActivePinia, setActivePinia } from 'pinia';
import { defineComponent, nextTick, ref } from 'vue';
import type { Ref } from 'vue';
import { mount } from '@vue/test-utils';

const flowHooks = vi.hoisted(() => ({
  fitView: vi.fn(),
  nodeDoubleClick:
    null as ((payload: { node: { id: string }; event: MouseEvent }) => void | Promise<void>) | null,
  setEdges: vi.fn(),
  setNodes: vi.fn(),
}));

vi.mock('elkjs/lib/elk-api', () => ({
  default: class {
    layout() {
      return Promise.resolve({ children: [], edges: [] });
    }
  },
}));

vi.mock('elkjs/lib/elk-worker.min.js?url', () => ({
  default: 'mock-elk-worker.js',
}));

vi.mock('@vue-flow/core', () => ({
  getSmoothStepPath: vi.fn(() => ['', 0, 0]),
  useVueFlow: () => ({
    onConnect: vi.fn(),
    onEdgeClick: vi.fn(),
    onEdgeDoubleClick: vi.fn(),
    onNodeClick: vi.fn(),
    onNodeDoubleClick: (callback: typeof flowHooks.nodeDoubleClick) => {
      flowHooks.nodeDoubleClick = callback;
    },
    onNodeDragStop: vi.fn(),
    onPaneClick: vi.fn(),
    fitView: flowHooks.fitView,
    getEdges: { value: [] },
    getNodes: { value: [] },
    setEdges: flowHooks.setEdges,
    setNodes: flowHooks.setNodes,
  }),
}));

import { useComposerCanvas } from '../useComposerCanvas';
import { useAppShellStore } from '../../stores/app-shell';
import { useSemanticZoomStore } from '../../stores/semantic-zoom';
import { useSessionStore } from '../../stores/session';
import type { PipelineDefinition, PipelineView } from '../../lib/pipeline-types';

function createEventTarget() {
  const nodeElement = document.createElement('div');
  nodeElement.className = 'vue-flow__node';
  vi.spyOn(nodeElement, 'getBoundingClientRect').mockReturnValue(new DOMRect(10, 20, 100, 60));

  const target = document.createElement('button');
  nodeElement.appendChild(target);
  document.body.appendChild(nodeElement);

  return { nodeElement, target };
}

function createMouseEvent(target: EventTarget): MouseEvent {
  return { target } as unknown as MouseEvent;
}

function createView(overrides?: Partial<PipelineView>): PipelineView {
  return {
    pipelineId: 'pipeline-1',
    status: 'paused',
    running: false,
    ready: [],
    active: [],
    completed: [],
    mermaid: 'graph TD;',
    edges: [],
    nodes: [
      {
        nodeId: 'prompt-1',
        sessionId: 'workflow-prompt-local',
        acpSessionId: 'acp-remote-1',
        provenance: {
          agent: 'architect',
          inspectable: true,
          detail: {
            sessionId: 'acp-remote-1',
            canReconnect: true,
          },
        },
        status: 'active',
        attempts: 1,
        type: 'prompt',
        prompt: 'Reconnect me',
        agent: 'architect',
      },
      {
        nodeId: 'tool-1',
        sessionId: 'workflow-tool-local',
        acpSessionId: 'tool-acp-ignored',
        provenance: {
          agent: null,
          inspectable: true,
          detail: {
            sessionId: 'tool-acp-ignored',
            canReconnect: false,
          },
        },
        status: 'completed',
        attempts: 1,
        type: 'tool',
        toolName: 'example.echo',
        toolProvenance: {
          toolName: 'example.echo',
          resolvedArgs: { value: 1 },
          responseSummary: '{"value":1}',
        },
      },
    ],
    ...overrides,
  };
}

function mountComposerCanvas(options?: {
  designMode?: boolean;
  definition?: PipelineDefinition;
  view?: PipelineView | null;
  layoutDirection?: Ref<'LR' | 'TB'>;
}) {
  const designMode = ref(options?.designMode ?? false);
  const definition = ref<PipelineDefinition>(options?.definition ?? { nodes: [], edges: [] });
  const view = ref<PipelineView | null | undefined>(options?.view ?? createView());
  const layoutDirection = options?.layoutDirection ?? ref<'LR' | 'TB'>('LR');

  mount(
    defineComponent({
      setup() {
        useComposerCanvas({
          designMode: () => designMode.value,
          localDefinition: () => definition.value,
          view: () => view.value,
          layoutDirection,
          updateDefinition: vi.fn(),
          selectNode: vi.fn(),
          selectEdge: vi.fn(),
        });
        return () => null;
      },
    }),
    {
      global: {
        plugins: [getActivePinia()!],
      },
    },
  );

  return { definition, designMode, layoutDirection, view };
}

async function flushComposerCanvas() {
  await nextTick();
  await nextTick();
}

describe('useComposerCanvas double-click behavior', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    flowHooks.nodeDoubleClick = null;
    flowHooks.fitView.mockReset();
    flowHooks.setEdges.mockReset();
    flowHooks.setNodes.mockReset();
    document.body.innerHTML = '';
    localStorage.clear();
  });

  // Removed in T-170.10: inline reconnect-on-double-click was moved to the
  // SemanticZoomOverlay/PipelineFlyout. handleNodeDoubleClick now only calls
  // enterInspect (or designNodeDoubleClick in design mode) — it does not call
  // focusOrResumeRemoteSession or change the active tab. Tests that asserted
  // those behaviors were removed because the production code no longer
  // implements them.

  it('preserves inspect zoom for tool nodes and never attempts reconnect', async () => {
    const appShellStore = useAppShellStore();
    const sessionStore = useSessionStore();
    const zoomStore = useSemanticZoomStore();
    appShellStore.setActiveTab('pipelines');

    const reconnectSpy = vi.spyOn(sessionStore, 'focusOrResumeRemoteSession');
    const inspectSpy = vi.spyOn(zoomStore, 'enterInspect');

    mountComposerCanvas({ view: createView() });
    const { target } = createEventTarget();

    await flowHooks.nodeDoubleClick?.({
      node: { id: 'tool-1' },
      event: createMouseEvent(target),
    });

    expect(reconnectSpy).not.toHaveBeenCalled();
    expect(inspectSpy).toHaveBeenCalledWith('tool-1', expect.any(DOMRect));
    expect(appShellStore.activeTab).toBe('pipelines');
  });

  it('honors provenance canReconnect even when a non-tool ACP session id exists', async () => {
    const sessionStore = useSessionStore();
    const zoomStore = useSemanticZoomStore();

    const reconnectSpy = vi.spyOn(sessionStore, 'focusOrResumeRemoteSession');
    const inspectSpy = vi.spyOn(zoomStore, 'enterInspect');

    mountComposerCanvas({
      view: createView({
        nodes: [
          {
            nodeId: 'prompt-1',
            sessionId: 'workflow-prompt-local',
            acpSessionId: 'acp-remote-1',
            provenance: {
              agent: 'architect',
              inspectable: true,
              detail: {
                sessionId: 'acp-remote-1',
                canReconnect: false,
              },
            },
            status: 'active',
            attempts: 1,
            type: 'prompt',
            prompt: 'Inspect me instead',
            agent: 'architect',
          },
        ],
      }),
    });
    const { target } = createEventTarget();

    await flowHooks.nodeDoubleClick?.({
      node: { id: 'prompt-1' },
      event: createMouseEvent(target),
    });

    expect(reconnectSpy).not.toHaveBeenCalled();
    expect(inspectSpy).toHaveBeenCalledWith('prompt-1', expect.any(DOMRect));
  });

  it('preserves inspect zoom when no ACP session is available', async () => {
    const sessionStore = useSessionStore();
    const zoomStore = useSemanticZoomStore();

    const reconnectSpy = vi.spyOn(sessionStore, 'focusOrResumeRemoteSession');
    const inspectSpy = vi.spyOn(zoomStore, 'enterInspect');

    mountComposerCanvas({
      view: createView({
        nodes: [
          {
            nodeId: 'prompt-1',
            sessionId: 'workflow-prompt-local',
            status: 'active',
            attempts: 1,
            type: 'prompt',
            prompt: 'Inspect me instead',
            agent: 'architect',
          },
        ],
      }),
    });
    const { target } = createEventTarget();

    await flowHooks.nodeDoubleClick?.({
      node: { id: 'prompt-1' },
      event: createMouseEvent(target),
    });

    expect(reconnectSpy).not.toHaveBeenCalled();
    expect(inspectSpy).toHaveBeenCalledWith('prompt-1', expect.any(DOMRect));
  });

  // Removed in T-170.10: see note above. The "reconnect cannot find ACP
  // session" path was part of the inline double-click reconnect logic that
  // no longer exists.
});

describe('useComposerCanvas runtime auto-fit behavior', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    flowHooks.nodeDoubleClick = null;
    flowHooks.fitView.mockReset();
    flowHooks.setEdges.mockReset();
    flowHooks.setNodes.mockReset();
    document.body.innerHTML = '';
    localStorage.clear();
  });

  it('fits once on the initial runtime load for a pipeline', async () => {
    mountComposerCanvas({ view: createView() });

    await flushComposerCanvas();

    expect(flowHooks.fitView).toHaveBeenCalledTimes(1);
    expect(flowHooks.fitView).toHaveBeenCalledWith({ padding: 0.15, duration: 200 });
  });

  it('does not refit when the same pipeline receives subsequent view updates', async () => {
    const { view } = mountComposerCanvas({ view: createView() });

    await flushComposerCanvas();
    flowHooks.fitView.mockClear();

    view.value = createView({ mermaid: 'graph LR;' });
    await flushComposerCanvas();

    expect(flowHooks.fitView).not.toHaveBeenCalled();
  });

  it('refits when runtime view switches to a different pipeline id', async () => {
    const { view } = mountComposerCanvas({ view: createView() });

    await flushComposerCanvas();
    flowHooks.fitView.mockClear();

    view.value = createView({ pipelineId: 'pipeline-2' });
    await flushComposerCanvas();

    expect(flowHooks.fitView).toHaveBeenCalledTimes(1);
    expect(flowHooks.fitView).toHaveBeenCalledWith({ padding: 0.15, duration: 200 });
  });

  it('clears the fit sentinel in design mode so a later runtime load can fit again', async () => {
    const { designMode, view } = mountComposerCanvas({ view: createView() });

    await flushComposerCanvas();
    flowHooks.fitView.mockClear();

    designMode.value = true;
    await flushComposerCanvas();

    designMode.value = false;
    view.value = createView({ mermaid: 'graph RL;' });
    await flushComposerCanvas();

    expect(flowHooks.fitView).toHaveBeenCalledTimes(1);
    expect(flowHooks.fitView).toHaveBeenCalledWith({ padding: 0.15, duration: 200 });
  });
});

describe('useComposerCanvas layout direction persistence', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    flowHooks.nodeDoubleClick = null;
    flowHooks.fitView.mockReset();
    flowHooks.setEdges.mockReset();
    flowHooks.setNodes.mockReset();
    document.body.innerHTML = '';
    localStorage.clear();
  });

  it('restores saved direction for the active pipeline session', async () => {
    localStorage.setItem('maenifold:pipeline:pipeline-1:direction', 'TB');
    const layoutDirection = ref<'LR' | 'TB'>('LR');

    mountComposerCanvas({ view: createView(), layoutDirection });
    await flushComposerCanvas();

    expect(layoutDirection.value).toBe('TB');
  });

  it('keeps separate layout directions per pipeline session', async () => {
    const layoutDirection = ref<'LR' | 'TB'>('LR');
    localStorage.setItem('maenifold:pipeline:pipeline-2:direction', 'LR');

    const { view } = mountComposerCanvas({ view: createView(), layoutDirection });
    await flushComposerCanvas();

    layoutDirection.value = 'TB';
    await nextTick();

    expect(localStorage.getItem('maenifold:pipeline:pipeline-1:direction')).toBe('TB');

    view.value = createView({ pipelineId: 'pipeline-2' });
    await flushComposerCanvas();

    expect(layoutDirection.value).toBe('LR');

    view.value = createView({ pipelineId: 'pipeline-1' });
    await flushComposerCanvas();

    expect(layoutDirection.value).toBe('TB');
  });
});
