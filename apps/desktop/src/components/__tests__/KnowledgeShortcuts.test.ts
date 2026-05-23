// @vitest-environment jsdom
import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { computed, defineComponent, h, inject, nextTick, provide, ref, type ComputedRef } from 'vue';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import KnowledgeWorkspace from '../KnowledgeWorkspace.vue';
import { useAppShellStore, type AppShellTab } from '../../stores/app-shell';
import { useKnowledgeStore, type KnowledgeSubtab } from '../../stores/knowledge';

const mockCallTool = vi.fn();
const mcpClient = () => ({
  connected: ref(true),
  connect: vi.fn(),
  disconnect: vi.fn(),
  error: ref(null),
  reconnecting: ref(false),
  onNotification: vi.fn(() => vi.fn()),
  onDisconnect: vi.fn(() => vi.fn()),
  callTool: mockCallTool,
  listTools: vi.fn(async () => []),
  supportsResourceSubscribe: vi.fn(() => false),
});

vi.mock('../../composables/useSharedMcpClient', () => ({
  getSharedMcpClient: vi.fn(() => ({
    ...mcpClient(),
    readResource: vi.fn(),
    subscribeResource: vi.fn(),
    unsubscribeResource: vi.fn(),
  })),
  usePipelineMcp: vi.fn(() => ({
    ...mcpClient(),
    callPipeline: vi.fn(),
    diffViews: vi.fn(() => []),
    listAssets: vi.fn(async () => []),
    listPipelines: vi.fn(async () => ({ pipelines: [] })),
    listWorkflows: vi.fn(async () => []),
    startPolling: vi.fn(() => vi.fn()),
  })),
}));

const dialogOpenKey = Symbol('dialog-open');
const slotStub = { template: '<div><slot /></div>' };

const dialogStubs = {
  Dialog: defineComponent({
    props: { open: { type: Boolean, default: false } },
    setup(props, { slots }) {
      provide(dialogOpenKey, computed(() => props.open));
      return () => h('div', { 'data-testid': 'dialog-root', 'data-open': String(props.open) }, slots.default?.());
    },
  }),
  DialogContent: defineComponent({
    setup(_props, { slots }) {
      const open = inject<ComputedRef<boolean>>(dialogOpenKey);
      return () => (open?.value ? h('div', { 'data-testid': 'dialog-content' }, slots.default?.()) : null);
    },
  }),
  DialogHeader: slotStub, DialogTitle: slotStub, DialogDescription: slotStub, DialogFooter: slotStub, DialogTrigger: slotStub,
};

const layoutStubs = {
  KnowledgeGraphView: { template: '<div data-testid="graph-view">Graph</div>' },
  KnowledgeLeafPane: slotStub,
  KnowledgeRelatedConcepts: slotStub,
  KnowledgeRowActions: slotStub,
  KnowledgeSubtabToggle: slotStub,
  KnowledgeTabStrip: slotStub,
  KnowledgeVaultNode: slotStub,
  ProvenanceBreadcrumbs: slotStub,
  ResizableHandle: slotStub,
  ResizablePanel: slotStub,
  ResizablePanelGroup: slotStub,
  ScrollArea: slotStub,
  ...dialogStubs,
};

function mountWorkspace(options: {
  activeTab?: AppShellTab;
  activeSubtab?: KnowledgeSubtab;
} = {}) {
  const pinia = createPinia();
  setActivePinia(pinia);
  const appShell = useAppShellStore();
  const knowledge = useKnowledgeStore();
  appShell.setActiveTab(options.activeTab ?? 'knowledge');
  knowledge.setActiveSubtab(options.activeSubtab ?? 'memory');
  const wrapper = mount(KnowledgeWorkspace, {
    attachTo: document.body,
    global: {
      plugins: [pinia],
      stubs: layoutStubs,
    },
  });
  return { appShell, knowledge, wrapper };
}

async function dispatchShortcut(key: string) {
  const event = new KeyboardEvent('keydown', {
    key,
    metaKey: true,
    bubbles: true,
    cancelable: true,
  });
  const preventDefault = vi.spyOn(event, 'preventDefault');
  window.dispatchEvent(event);
  await nextTick();
  await nextTick();
  return { preventDefault };
}

describe('KnowledgeWorkspace shortcuts', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
    localStorage.clear();
    mockCallTool.mockReset().mockResolvedValue('### Folders\n\n### Files');
  });

  it('Cmd+F focuses the Memory search input when Knowledge tab is active', async () => {
    const { wrapper } = mountWorkspace();
    const search = wrapper.find<HTMLInputElement>('[data-testid="knowledge-memory-search"]');
    expect(search.exists()).toBe(true);

    const { preventDefault } = await dispatchShortcut('f');

    expect(preventDefault).toHaveBeenCalledOnce();
    expect(document.activeElement).toBe(search.element);
    wrapper.unmount();
  });

  it('Cmd+N opens the new-note dialog when Knowledge tab is active', async () => {
    const { wrapper } = mountWorkspace();
    expect(wrapper.find('[data-testid="new-note-form"]').exists()).toBe(false);

    const { preventDefault } = await dispatchShortcut('n');

    expect(preventDefault).toHaveBeenCalledOnce();
    expect(wrapper.find('[data-testid="new-note-form"]').exists()).toBe(true);
    wrapper.unmount();
  });

  it('does not fire either shortcut when the active app tab is not Knowledge', async () => {
    const { knowledge, wrapper } = mountWorkspace({ activeTab: 'home' });
    const search = wrapper.find<HTMLInputElement>('[data-testid="knowledge-memory-search"]');
    expect(search.exists()).toBe(true);

    const findShortcut = await dispatchShortcut('f');
    const newShortcut = await dispatchShortcut('n');

    expect(findShortcut.preventDefault).not.toHaveBeenCalled();
    expect(newShortcut.preventDefault).not.toHaveBeenCalled();
    expect(document.activeElement).not.toBe(search.element);
    expect(wrapper.find('[data-testid="new-note-form"]').exists()).toBe(false);
    expect(knowledge.activeSubtab).toBe('memory');
    wrapper.unmount();
  });

  it("Cmd+F switches from Graph to Memory before focusing search", async () => {
    const { knowledge, wrapper } = mountWorkspace({ activeSubtab: 'graph' });
    expect(wrapper.find('[data-testid="graph-view"]').exists()).toBe(true);

    const { preventDefault } = await dispatchShortcut('f');
    const search = wrapper.find<HTMLInputElement>('[data-testid="knowledge-memory-search"]');

    expect(preventDefault).toHaveBeenCalledOnce();
    expect(knowledge.activeSubtab).toBe('memory');
    expect(search.exists()).toBe(true);
    expect(document.activeElement).toBe(search.element);
    wrapper.unmount();
  });
});
