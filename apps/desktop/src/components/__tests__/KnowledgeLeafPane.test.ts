// @vitest-environment jsdom
import { flushPromises, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { nextTick, ref } from 'vue';
import KnowledgeLeafPane from '../KnowledgeLeafPane.vue';
import { useKnowledgeStore, type KnowledgeTabCacheEntry } from '../../stores/knowledge';
import type { MemoryFile } from '../../lib/workflow-types';

const mockCallTool = vi.fn();

vi.mock('../../composables/usePipelineMcp', () => ({
  getSharedMcpClient: vi.fn(() => ({
    connected: ref(true),
    connect: vi.fn(),
    error: ref(null),
    onNotification: vi.fn(() => vi.fn()),
    onDisconnect: vi.fn(() => vi.fn()),
    callTool: mockCallTool,
  })),
}));

type PollerCallbacks = {
  onTreeChanged: () => void;
  onActiveNoteChanged: (note: MemoryFile) => void;
};

let capturedCallbacks: PollerCallbacks | null = null;
const mockPollerStart = vi.fn();
const mockPollerStop = vi.fn();

vi.mock('../../composables/useKnowledgePoller', () => ({
  createKnowledgePoller: vi.fn((_deps: unknown, cbs: PollerCallbacks) => {
    capturedCallbacks = cbs;
    return { start: mockPollerStart, stop: mockPollerStop };
  }),
}));

function makeNote(overrides: Partial<MemoryFile> = {}): MemoryFile {
  return {
    uri: 'memory://test/note',
    title: 'Test Note',
    content: 'Original content.',
    checksum: 'cs1',
    created: null,
    modified: '2024-01-01T00:00:00Z',
    ...overrides,
  };
}

function seedActiveNote(store: ReturnType<typeof useKnowledgeStore>, note: MemoryFile) {
  const entry: KnowledgeTabCacheEntry = { title: note.title, loading: false, error: null, content: note };
  store.tabCache = new Map([[note.uri, entry]]);
  store.openTabs = [note.uri];
  store.activeTabUri = note.uri;
}

const sharedStubs = {
  KnowledgeInlineEditor: { template: '<div data-testid="editor-stub" />' },
  MarkdownView: true,
  ScrollArea: { template: '<div><slot /></div>' },
};

describe('KnowledgeLeafPane external-change banner', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockCallTool.mockReset().mockResolvedValue({ success: true, data: { items: [] } });
    mockPollerStart.mockReset();
    mockPollerStop.mockReset();
    capturedCallbacks = null;
  });

  it('starts the poller on mount and stops it on unmount', async () => {
    const wrapper = mount(KnowledgeLeafPane, { global: { stubs: sharedStubs } });
    await nextTick();
    expect(mockPollerStart).toHaveBeenCalledOnce();
    wrapper.unmount();
    expect(mockPollerStop).toHaveBeenCalledOnce();
  });

  it('shows the external-change banner when editing and active note changes', async () => {
    const store = useKnowledgeStore();
    seedActiveNote(store, makeNote());

    const wrapper = mount(KnowledgeLeafPane, { global: { stubs: sharedStubs } });
    await flushPromises();

    const editBtn = wrapper.findAll('button').find((b) => b.text().includes('Edit'));
    expect(editBtn).toBeDefined();
    await editBtn!.trigger('click');
    await nextTick();

    expect(capturedCallbacks).not.toBeNull();
    capturedCallbacks!.onActiveNoteChanged(makeNote({ checksum: 'cs2' }));
    await nextTick();

    const banner = wrapper.find('[data-testid="external-change-banner"]');
    expect(banner.exists()).toBe(true);
    expect(banner.text()).toContain('Note changed externally');
    // Editor stub must still be rendered (draft preserved)
    expect(wrapper.find('[data-testid="editor-stub"]').exists()).toBe(true);

    wrapper.unmount();
  });

  it('preserves editing mode (does not call reloadNote) while banner is shown', async () => {
    const store = useKnowledgeStore();
    seedActiveNote(store, makeNote());
    const reloadNoteSpy = vi.spyOn(store, 'reloadNote').mockResolvedValue();

    const wrapper = mount(KnowledgeLeafPane, { global: { stubs: sharedStubs } });
    await flushPromises();

    const editBtn = wrapper.findAll('button').find((b) => b.text().includes('Edit'));
    await editBtn!.trigger('click');
    await nextTick();

    capturedCallbacks!.onActiveNoteChanged(makeNote({ checksum: 'cs2' }));
    await nextTick();

    expect(reloadNoteSpy).not.toHaveBeenCalled();
    expect(wrapper.find('[data-testid="editor-stub"]').exists()).toBe(true);

    wrapper.unmount();
  });

  it('silently calls reloadNote when not editing and note changes', async () => {
    const store = useKnowledgeStore();
    seedActiveNote(store, makeNote());
    const reloadNoteSpy = vi.spyOn(store, 'reloadNote').mockResolvedValue();

    const wrapper = mount(KnowledgeLeafPane, { global: { stubs: sharedStubs } });
    await flushPromises();
    // editing is false by default

    capturedCallbacks!.onActiveNoteChanged(makeNote({ checksum: 'cs2' }));
    await nextTick();

    expect(reloadNoteSpy).toHaveBeenCalledWith('memory://test/note');
    expect(wrapper.find('[data-testid="external-change-banner"]').exists()).toBe(false);

    wrapper.unmount();
  });
});
