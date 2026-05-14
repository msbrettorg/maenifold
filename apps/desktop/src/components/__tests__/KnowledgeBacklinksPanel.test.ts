// @vitest-environment jsdom
import { flushPromises, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { nextTick } from 'vue';
import KnowledgeBacklinksPanel from '../KnowledgeBacklinksPanel.vue';
import { useKnowledgeStore, type KnowledgeTabCacheEntry } from '../../stores/knowledge';
import type { MemoryFile, SearchResult } from '../../lib/workflow-types';

const mockApi = vi.hoisted(() => ({ searchMemories: vi.fn() }));
vi.mock('../../composables/useMemoryApi', () => ({ useMemoryApi: vi.fn(() => mockApi) }));

const stubs = { Button: { template: '<button type="button"><slot /></button>' },
  ScrollArea: { template: '<div data-testid="backlinks-scroll-area"><slot /></div>' }, Skeleton: { template: '<div />' },
  Alert: { template: '<div role="alert"><slot /></div>' }, AlertTitle: { template: '<strong><slot /></strong>' },
  AlertDescription: { template: '<span><slot /></span>' } };

function search(results: SearchResult['results'] = []): SearchResult {
  return { results, totalCount: results.length, page: 1, pageSize: 30 };
}

function note(uri: string, title: string): MemoryFile {
  return { uri, title, content: `[[${title}]]`, checksum: 'cs1', created: null, modified: null };
}

function seed(store: ReturnType<typeof useKnowledgeStore>, active: MemoryFile): void {
  const entry: KnowledgeTabCacheEntry = { title: active.title, loading: false, error: null, content: active };
  store.tabCache = new Map([[active.uri, entry]]);
  store.openTabs = [active.uri];
  store.activeTabUri = active.uri;
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
}

describe('KnowledgeBacklinksPanel', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockApi.searchMemories.mockReset();
  });

  it('renders header, scroll area, loading state, empty state, and FullText query', async () => {
    const pending = deferred<SearchResult>();
    mockApi.searchMemories.mockReturnValueOnce(pending.promise);
    seed(useKnowledgeStore(), note('memory://active', 'Active Note'));
    const wrapper = mount(KnowledgeBacklinksPanel, { global: { stubs } });
    await nextTick();
    expect(wrapper.text()).toContain('Linked from');
    expect(wrapper.find('[data-testid="backlinks-scroll-area"]').exists()).toBe(true);
    expect(wrapper.text()).toContain('Loading backlinks…');
    pending.resolve(search());
    await flushPromises();
    expect(mockApi.searchMemories).toHaveBeenCalledWith('[[Active Note]]', {
      mode: 'FullText',
      pageSize: 30,
    });
    expect(wrapper.text()).toContain('No notes link to this one yet.');
  });

  it('renders an error state when search fails', async () => {
    mockApi.searchMemories.mockRejectedValueOnce(new Error('search unavailable'));
    seed(useKnowledgeStore(), note('memory://active', 'Active Note'));
    const wrapper = mount(KnowledgeBacklinksPanel, { global: { stubs } });
    await flushPromises();
    expect(wrapper.text()).toContain('Could not load backlinks');
    expect(wrapper.text()).toContain('search unavailable');
  });

  it('renders rows from search, filters active note, and opens clicked source', async () => {
    mockApi.searchMemories.mockResolvedValueOnce(search([
      { uri: 'memory://source/Note', title: 'Source Note', snippet: 'Links to [[Active Note]].', score: 1 },
      { uri: 'memory://active', title: 'Active Note', snippet: 'Self [[Active Note]].', score: 1 },
    ]));
    const store = useKnowledgeStore();
    seed(store, note('memory://active', 'Active Note'));
    const openNoteSpy = vi.spyOn(store, 'openNote').mockImplementation(() => undefined);
    const wrapper = mount(KnowledgeBacklinksPanel, { global: { stubs } });
    await flushPromises();
    expect(wrapper.findAll('[data-testid="backlink-row"]')).toHaveLength(1);
    expect(wrapper.text()).toContain('Source Note');
    await wrapper.find('[data-testid="backlink-row"]').trigger('click');
    expect(openNoteSpy).toHaveBeenCalledWith('memory://source/Note');
  });

  it('reloads backlinks when activeTabUri changes', async () => {
    mockApi.searchMemories.mockResolvedValue(search());
    const store = useKnowledgeStore();
    seed(store, note('memory://first', 'First Note'));
    mount(KnowledgeBacklinksPanel, { global: { stubs } });
    await flushPromises();
    const second = note('memory://second', 'Second Note');
    store.tabCache = new Map([...store.tabCache, [second.uri, { title: second.title, loading: false, error: null, content: second }]]);
    store.activeTabUri = second.uri;
    await flushPromises();
    expect(mockApi.searchMemories).toHaveBeenCalledWith('[[First Note]]', expect.any(Object));
    expect(mockApi.searchMemories).toHaveBeenCalledWith('[[Second Note]]', expect.any(Object));
  });
});
