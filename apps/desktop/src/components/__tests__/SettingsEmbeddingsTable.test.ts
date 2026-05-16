// @vitest-environment jsdom
import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import SettingsEmbeddingsTable from '../SettingsEmbeddingsTable.vue';
import type { ModelInfo, PendingAction } from '../../composables/useEmbeddingsActions';

const models: ModelInfo[] = [
  {
    id: 'bge-m3',
    name: 'BGE-M3',
    dimensions: 1024,
    maxTokens: 8192,
    sizeBytes: 2_000_000_000,
    license: 'MIT',
    huggingfaceRepo: 'BAAI/bge-m3',
    installed: true,
    isActive: true,
  },
  {
    id: 'gte-large',
    name: 'GTE-Large',
    dimensions: 1024,
    maxTokens: 512,
    sizeBytes: 1_300_000_000,
    license: 'MIT',
    huggingfaceRepo: 'thenlper/gte-large',
    installed: true,
    isActive: false,
  },
  {
    id: 'all-minilm',
    name: 'all-MiniLM-L6',
    dimensions: 384,
    maxTokens: 256,
    sizeBytes: 80_000_000,
    license: 'Apache-2.0',
    huggingfaceRepo: 'sentence-transformers/all-MiniLM-L6-v2',
    installed: false,
    isActive: false,
  },
  {
    id: 'nomic-embed',
    name: 'nomic-embed-text',
    dimensions: 768,
    maxTokens: 8192,
    sizeBytes: 548_000_000,
    license: 'Apache-2.0',
    huggingfaceRepo: 'nomic-ai/nomic-embed-text-v1.5',
    installed: true,
    isActive: false,
  },
  {
    id: 'e5-large',
    name: 'E5-large-v2',
    dimensions: 1024,
    maxTokens: 512,
    sizeBytes: 1_300_000_000,
    license: 'MIT',
    huggingfaceRepo: 'intfloat/e5-large-v2',
    installed: false,
    isActive: false,
    mtebRetrieval: 0.551,
  },
];

const pending: Record<string, PendingAction | undefined> = {};

const mountTable = (sortColumn = '', sortDirection: 'asc' | 'desc' | 'none' = 'none') => mount(
  SettingsEmbeddingsTable,
  {
    props: {
      models,
      pending,
      sortColumn,
      sortDirection,
    },
  },
);

const headerByText = (wrapper: ReturnType<typeof mountTable>, label: string) => wrapper.findAll('th')
  .find((header) => header.text().includes(label));

const sortButtonByText = (wrapper: ReturnType<typeof mountTable>, label: string) => wrapper.findAll('button')
  .find((button) => button.text().includes(label));

describe('SettingsEmbeddingsTable', () => {
  it('renders the sortable embeddings model fixture', () => {
    const wrapper = mountTable();

    expect(wrapper.findAll('tbody tr')).toHaveLength(5);
    expect(wrapper.text()).toContain('BGE-M3');
    expect(wrapper.text()).toContain('all-MiniLM-L6');
    expect(wrapper.text()).toContain('nomic-embed-text');
    expect(wrapper.text()).toContain('E5-large-v2');
  });

  it('emits sort requests for each sortable column header', async () => {
    const wrapper = mountTable();
    const columns = [
      ['Model', 'model'],
      ['Specs', 'specs'],
      ['Quality', 'quality'],
      ['Status', 'status'],
    ] as const;

    for (const [label] of columns) {
      const button = sortButtonByText(wrapper, label);
      expect(button?.exists()).toBe(true);
      await button?.trigger('click');
    }

    expect(wrapper.emitted('sort')).toEqual(columns.map(([, column]) => [column]));
  });

  it('sets ascending aria-sort on the active sort column', () => {
    const wrapper = mountTable('quality', 'asc');

    expect(headerByText(wrapper, 'Quality')?.attributes('aria-sort')).toBe('ascending');
    expect(headerByText(wrapper, 'Model')?.attributes('aria-sort')).toBe('none');
    expect(headerByText(wrapper, 'Specs')?.attributes('aria-sort')).toBe('none');
    expect(headerByText(wrapper, 'Status')?.attributes('aria-sort')).toBe('none');
  });

  it('sets descending aria-sort on the active sort column', () => {
    const wrapper = mountTable('status', 'desc');

    expect(headerByText(wrapper, 'Status')?.attributes('aria-sort')).toBe('descending');
    expect(headerByText(wrapper, 'Model')?.attributes('aria-sort')).toBe('none');
    expect(headerByText(wrapper, 'Specs')?.attributes('aria-sort')).toBe('none');
    expect(headerByText(wrapper, 'Quality')?.attributes('aria-sort')).toBe('none');
  });

  it('sets aria-sort none when no header matches the active sort column', () => {
    const wrapper = mountTable('missing-column', 'asc');

    expect(headerByText(wrapper, 'Model')?.attributes('aria-sort')).toBe('none');
    expect(headerByText(wrapper, 'Specs')?.attributes('aria-sort')).toBe('none');
    expect(headerByText(wrapper, 'Quality')?.attributes('aria-sort')).toBe('none');
    expect(headerByText(wrapper, 'Status')?.attributes('aria-sort')).toBe('none');
  });
});
