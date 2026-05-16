// @vitest-environment jsdom
import { flushPromises, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ref } from 'vue';
import KnowledgeVaultBrowser from '../KnowledgeVaultBrowser.vue';
import { useKnowledgeStore } from '../../stores/knowledge';
import type { KnowledgeFileEntry, KnowledgeFolderNode } from '../../lib/knowledge-helpers';
import type { MemorySearchItem } from '../../lib/workflow-types';

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

type SortableFile = KnowledgeFileEntry & { modifiedAt: string };
type SortableFolder = KnowledgeFolderNode & { modifiedAt: string };
type SortableSearchResult = MemorySearchItem & { modifiedAt: string };

const fixtureFiles: SortableFile[] = [
  { name: 'banana.md', path: 'banana.md', uri: 'memory://banana', modifiedAt: '2026-05-02T12:00:00Z' },
  { name: 'apple.md', path: 'apple.md', uri: 'memory://apple', modifiedAt: '2026-04-01T12:00:00Z' },
  { name: 'cherry.md', path: 'cherry.md', uri: 'memory://cherry', modifiedAt: '2026-05-01T12:00:00Z' },
];

const fixtureFolders: SortableFolder[] = [
  {
    name: 'zeta-folder', path: 'zeta-folder', fileCount: 1,
    open: false, loaded: true, loading: false, error: null,
    children: [], files: [{ name: 'z-note.md', path: 'zeta-folder/z-note.md', uri: 'memory://zeta-folder/z-note' }],
    modifiedAt: '2026-05-03T12:00:00Z',
  },
  {
    name: 'alpha-folder', path: 'alpha-folder', fileCount: 1,
    open: false, loaded: true, loading: false, error: null,
    children: [], files: [{ name: 'a-note.md', path: 'alpha-folder/a-note.md', uri: 'memory://alpha-folder/a-note' }],
    modifiedAt: '2026-05-04T12:00:00Z',
  },
];

const fixtureSearchResults: SortableSearchResult[] = [
  { uri: 'memory://cherry', title: 'cherry.md', snippet: 'cherry content', score: 0.9, modifiedAt: '2026-05-03T09:00:00Z' },
  { uri: 'memory://apple', title: 'apple.md', snippet: 'apple content', score: 0.5, modifiedAt: '2026-05-01T09:00:00Z' },
  { uri: 'memory://banana', title: 'banana.md', snippet: 'banana content', score: 0.7, modifiedAt: '2026-05-04T09:00:00Z' },
];

const treeNames = new Set([...fixtureFiles.map((file) => file.name), ...fixtureFolders.map((folder) => folder.name)]);
const searchTitles = new Set(fixtureSearchResults.map((result) => result.title));

const uiStubs = {
  Alert: { template: '<div><slot /></div>' },
  AlertDescription: { template: '<div><slot /></div>' },
  AlertTitle: { template: '<div><slot /></div>' },
  Button: {
    template: '<button type="button" v-bind="$attrs" @click="$emit(\'click\', $event)"><slot /></button>',
  },
  Input: {
    props: ['modelValue'],
    emits: ['update:modelValue', 'keydown'],
    template: '<input :value="modelValue" v-bind="$attrs" @input="$emit(\'update:modelValue\', $event.target.value)" @keydown="$emit(\'keydown\', $event)" />',
  },
  ScrollArea: { template: '<div><slot /></div>' },
  Skeleton: { template: '<div data-testid="skeleton-stub" />' },
  Tabs: {
    emits: ['update:modelValue'],
    template: '<div data-testid="tabs-stub"><slot /></div>',
  },
  TabsList: { template: '<div><slot /></div>' },
  TabsTrigger: {
    props: ['value'],
    template: '<button type="button" :data-testid="`vault-tab-${value}`" @click="$parent?.$parent?.$emit(\'update:modelValue\', value)"><slot /></button>',
  },
  KnowledgeVaultNode: {
    props: ['node'],
    template: '<div data-testid="vault-folder-row">{{ node.name }}</div>',
  },
  KnowledgeRowActions: {
    template: '<div data-testid="row-actions-stub" />',
  },
};

function seedStore(): void {
  const store = useKnowledgeStore();
  store.rootFiles = fixtureFiles as KnowledgeFileEntry[];
  store.folders = fixtureFolders as KnowledgeFolderNode[];
  store.searchResults = fixtureSearchResults as MemorySearchItem[];
  store.searchQuery = 'fruit';
}

async function mountBrowser() {
  seedStore();
  const wrapper = mount(KnowledgeVaultBrowser, {
    global: { stubs: uiStubs },
  });
  await flushPromises();
  return wrapper;
}

function getSortControl(wrapper: Awaited<ReturnType<typeof mountBrowser>>) {
  const byTestId = wrapper.find('[data-testid="vault-sort-select"]');
  return byTestId.exists() ? byTestId : wrapper.find('select');
}

async function setSort(wrapper: Awaited<ReturnType<typeof mountBrowser>>, value: string): Promise<void> {
  const control = getSortControl(wrapper);
  expect(control.exists()).toBe(true);
  await control.setValue(value);
  await flushPromises();
}

async function openSearchMode(wrapper: Awaited<ReturnType<typeof mountBrowser>>): Promise<void> {
  await wrapper.find('[data-testid="vault-tab-search"]').trigger('click');
  await flushPromises();
}

async function openTreeMode(wrapper: Awaited<ReturnType<typeof mountBrowser>>): Promise<void> {
  await wrapper.find('[data-testid="vault-tab-tree"]').trigger('click');
  await flushPromises();
}

function renderedTreeEntries(wrapper: Awaited<ReturnType<typeof mountBrowser>>): string[] {
  return wrapper
    .findAll('button, [data-testid="vault-folder-row"]')
    .map((node) => node.text().trim())
    .filter((text) => treeNames.has(text));
}

function renderedSearchEntries(wrapper: Awaited<ReturnType<typeof mountBrowser>>): string[] {
  return wrapper
    .findAll('button')
    .map((node) => node.text().trim())
    .filter((text) => searchTitles.has(text));
}

describe('KnowledgeVaultBrowser sort controls', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockCallTool.mockReset();
  });

  describe('tree mode', () => {
    it('defaults to Name (A-Z) sort', async () => {
      const wrapper = await mountBrowser();
      const control = getSortControl(wrapper);

      expect(control.exists()).toBe(true);
      expect((control.element as HTMLSelectElement).value).toBe('name-asc');
      expect(renderedTreeEntries(wrapper)).toEqual([
        'alpha-folder',
        'zeta-folder',
        'apple.md',
        'banana.md',
        'cherry.md',
      ]);
    });

    it('Name (Z-A) reverses file and folder order', async () => {
      const wrapper = await mountBrowser();

      await setSort(wrapper, 'name-desc');

      expect(renderedTreeEntries(wrapper)).toEqual([
        'zeta-folder',
        'alpha-folder',
        'cherry.md',
        'banana.md',
        'apple.md',
      ]);
    });

    it('Modified (newest) sorts by modification date', async () => {
      const wrapper = await mountBrowser();

      await setSort(wrapper, 'modified-newest');

      expect(renderedTreeEntries(wrapper)).toEqual([
        'alpha-folder',
        'zeta-folder',
        'banana.md',
        'cherry.md',
        'apple.md',
      ]);
    });

    it('folders appear before root files in all sort modes', async () => {
      const wrapper = await mountBrowser();

      for (const mode of ['name-asc', 'name-desc', 'modified-newest']) {
        await setSort(wrapper, mode);
        expect(renderedTreeEntries(wrapper).slice(0, 2)).toEqual([
          mode === 'name-desc' ? 'zeta-folder' : 'alpha-folder',
          mode === 'name-desc' ? 'alpha-folder' : 'zeta-folder',
        ]);
      }
    });
  });

  describe('search mode', () => {
    it('defaults to Relevance sort', async () => {
      const wrapper = await mountBrowser();

      await openSearchMode(wrapper);

      const control = getSortControl(wrapper);
      expect(control.exists()).toBe(true);
      expect((control.element as HTMLSelectElement).value).toBe('relevance');
      expect(renderedSearchEntries(wrapper)).toEqual(['cherry.md', 'banana.md', 'apple.md']);
    });

    it('Name sort orders results alphabetically', async () => {
      const wrapper = await mountBrowser();

      await openSearchMode(wrapper);
      await setSort(wrapper, 'name');

      expect(renderedSearchEntries(wrapper)).toEqual(['apple.md', 'banana.md', 'cherry.md']);
    });

    it('Modified sort orders results by date', async () => {
      const wrapper = await mountBrowser();

      await openSearchMode(wrapper);
      await setSort(wrapper, 'modified');

      expect(renderedSearchEntries(wrapper)).toEqual(['banana.md', 'cherry.md', 'apple.md']);
    });
  });

  describe('persistence', () => {
    it('remembers sort preference per mode within session', async () => {
      const wrapper = await mountBrowser();

      await setSort(wrapper, 'name-desc');
      await openSearchMode(wrapper);

      const searchControl = getSortControl(wrapper);
      expect(searchControl.exists()).toBe(true);
      expect((searchControl.element as HTMLSelectElement).value).toBe('relevance');

      await openTreeMode(wrapper);

      const treeControl = getSortControl(wrapper);
      expect(treeControl.exists()).toBe(true);
      expect((treeControl.element as HTMLSelectElement).value).toBe('name-desc');
      expect(renderedTreeEntries(wrapper)).toEqual([
        'zeta-folder',
        'alpha-folder',
        'cherry.md',
        'banana.md',
        'apple.md',
      ]);
    });
  });
});
