// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, getActivePinia, setActivePinia } from 'pinia';
import { defineComponent, nextTick } from 'vue';
import { mount } from '@vue/test-utils';

const flowHooks = vi.hoisted(() => ({
  nodeDoubleClick:
    null as ((payload: { node: { id: string }; event: MouseEvent }) => void | Promise<void>) | null,
}));

vi.mock('@vue-flow/core', () => ({
  getSmoothStepPath: vi.fn(() => ['', 0, 0]),
  useVueFlow: () => ({
    onConnect: vi.fn(),
    onEdgeClick: vi.fn(),
    onNodeClick: vi.fn(),
    onNodeDoubleClick: (callback: typeof flowHooks.nodeDoubleClick) => {
      flowHooks.nodeDoubleClick = callback;
    },
    onNodeDragStop: vi.fn(),
    onPaneClick: vi.fn(),
    fitView: vi.fn(),
    getEdges: { value: [] },
    getNodes: { value: [] },
    setEdges: vi.fn(),
    setNodes: vi.fn(),
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

function mountComposerCanvas(view: PipelineView) {
  const definition: PipelineDefinition = { nodes: [], edges: [] };
  mount(
    defineComponent({
      setup() {
        useComposerCanvas({
          designMode: () => false,
          localDefinition: () => definition,
          view: () => view,
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
}

describe('useComposerCanvas double-click behavior', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    flowHooks.nodeDoubleClick = null;
    document.body.innerHTML = '';
  });

  it('reconnects non-tool nodes by ACP session and switches to the Agents tab', async () => {
    const appShellStore = useAppShellStore();
    const sessionStore = useSessionStore();
    const zoomStore = useSemanticZoomStore();
    appShellStore.setActiveTab('pipelines');

    const reconnectSpy = vi
      .spyOn(sessionStore, 'focusOrResumeRemoteSession')
      .mockResolvedValue({ status: 'resumed', localSessionId: 'local-1' });
    const inspectSpy = vi.spyOn(zoomStore, 'enterInspect');

    mountComposerCanvas(createView());
    const { target } = createEventTarget();

    await flowHooks.nodeDoubleClick?.({
      node: { id: 'prompt-1' },
      event: createMouseEvent(target),
    });
    await nextTick();

    expect(reconnectSpy).toHaveBeenCalledWith('acp-remote-1');
    expect(appShellStore.activeTab).toBe('agents');
    expect(inspectSpy).not.toHaveBeenCalled();
  });

  it('preserves inspect zoom for tool nodes and never attempts reconnect', async () => {
    const appShellStore = useAppShellStore();
    const sessionStore = useSessionStore();
    const zoomStore = useSemanticZoomStore();
    appShellStore.setActiveTab('pipelines');

    const reconnectSpy = vi.spyOn(sessionStore, 'focusOrResumeRemoteSession');
    const inspectSpy = vi.spyOn(zoomStore, 'enterInspect');

    mountComposerCanvas(createView());
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

    mountComposerCanvas(
      createView({
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
    );
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

    mountComposerCanvas(
      createView({
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
    );
    const { target } = createEventTarget();

    await flowHooks.nodeDoubleClick?.({
      node: { id: 'prompt-1' },
      event: createMouseEvent(target),
    });

    expect(reconnectSpy).not.toHaveBeenCalled();
    expect(inspectSpy).toHaveBeenCalledWith('prompt-1', expect.any(DOMRect));
  });

  it('stays stable when reconnect cannot find the ACP session', async () => {
    const appShellStore = useAppShellStore();
    const sessionStore = useSessionStore();
    const zoomStore = useSemanticZoomStore();
    appShellStore.setActiveTab('pipelines');

    vi.spyOn(sessionStore, 'focusOrResumeRemoteSession').mockResolvedValue({ status: 'missing' });
    const inspectSpy = vi.spyOn(zoomStore, 'enterInspect');

    mountComposerCanvas(createView());
    const { target } = createEventTarget();

    await flowHooks.nodeDoubleClick?.({
      node: { id: 'prompt-1' },
      event: createMouseEvent(target),
    });
    await nextTick();

    expect(appShellStore.activeTab).toBe('pipelines');
    expect(inspectSpy).not.toHaveBeenCalled();
  });
});
