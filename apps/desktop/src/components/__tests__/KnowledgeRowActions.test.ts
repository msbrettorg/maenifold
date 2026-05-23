// @vitest-environment jsdom
import { flushPromises, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ref } from 'vue';
import KnowledgeRowActions from '../KnowledgeRowActions.vue';
import { useKnowledgeStore, type KnowledgeFileEntry } from '../../stores/knowledge';

const mockGatewayExecute = vi.fn();

vi.mock('../../composables/useSharedMcpClient', () => ({
  getSharedMcpClient: vi.fn(() => ({
    connected: ref(true),
    connect: vi.fn(),
    error: ref(null),
    onNotification: vi.fn(() => vi.fn()),
    onDisconnect: vi.fn(() => vi.fn()),
    gatewayExecute: mockGatewayExecute,
  })),
}));

const dialogStubs = {
  Dialog: { template: '<div><slot /></div>', props: ['open'] },
  DialogContent: { template: '<div><slot /></div>' },
  DialogHeader: { template: '<div><slot /></div>' },
  DialogTitle: { template: '<h2><slot /></h2>' },
  DialogDescription: { template: '<p><slot /></p>' },
  DialogFooter: { template: '<div><slot /></div>' },
};

const dropdownStubs = {
  DropdownMenu: { template: '<div><slot /></div>' },
  DropdownMenuTrigger: { template: '<div><slot /></div>' },
  DropdownMenuContent: { template: '<div><slot /></div>' },
  DropdownMenuItem: { template: '<div v-bind="$attrs" @click="$emit(\'select\', $event)"><slot /></div>' },
};

const selectStubs = {
  Select: {
    props: ['modelValue'],
    emits: ['update:modelValue'],
    template: '<div data-testid="select-stub"><slot /></div>',
  },
  SelectTrigger: { template: '<button type="button" data-testid="folder-picker-trigger"><slot /></button>' },
  SelectValue: { props: ['placeholder'], template: '<span>{{ placeholder }}</span>' },
  SelectContent: { template: '<div><slot /></div>' },
  SelectItem: {
    props: ['value'],
    template: '<button type="button" :data-testid="`folder-picker-option-${value}`" @click="$parent.$emit(\'update:modelValue\', value)"><slot /></button>',
  },
};

function file(): KnowledgeFileEntry {
  return {
    name: 'editor.md',
    path: 'research/editor.md',
    uri: 'memory://research/editor',
  };
}

describe('KnowledgeRowActions', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockGatewayExecute.mockReset();
    mockGatewayExecute.mockResolvedValue('### Folders\n\n### Files');
  });

  it('confirms delete with the note name and calls DeleteMemory with confirm true', async () => {
    const wrapper = mount(KnowledgeRowActions, {
      props: { file: file() },
      global: { stubs: { ...dialogStubs, ...dropdownStubs } },
    });

    await wrapper.find('[data-testid="row-action-delete"]').trigger('click');
    expect(wrapper.text()).toContain('This will permanently delete editor.md.');

    await wrapper.find('[data-testid="confirm-delete-note"]').trigger('click');
    expect(mockGatewayExecute).toHaveBeenCalledWith('maenifold.memory', 'DeleteMemory', {
      identifier: 'memory://research/editor',
      confirm: true,
    });
  });

  it('renames notes with MoveMemory in the same folder', async () => {
    const wrapper = mount(KnowledgeRowActions, {
      props: { file: file() },
      global: { stubs: { ...dialogStubs, ...dropdownStubs } },
    });

    await wrapper.find('[data-testid="row-action-rename"]').trigger('click');
    await wrapper.find('[data-testid="rename-note-input"]').setValue('renamed.md');
    await wrapper.find('[data-testid="confirm-rename-note"]').trigger('click');

    expect(mockGatewayExecute).toHaveBeenCalledWith('maenifold.memory', 'MoveMemory', {
      source: 'memory://research/editor',
      destination: 'research/renamed.md',
    });
  });

  it('moves notes with MoveMemory into the selected folder', async () => {
    const store = useKnowledgeStore();
    store.folders = [{
      name: 'archive',
      path: 'archive',
      fileCount: 0,
      open: false,
      loaded: true,
      loading: false,
      error: null,
      children: [],
      files: [],
    }];
    const wrapper = mount(KnowledgeRowActions, {
      props: { file: file() },
      global: { stubs: { ...dialogStubs, ...dropdownStubs, ...selectStubs } },
    });

    await wrapper.find('[data-testid="row-action-move"]').trigger('click');
    expect(wrapper.find('[data-testid="folder-picker-trigger"]').exists()).toBe(true);
    expect(wrapper.text()).toContain('archive');
    await wrapper.find('[data-testid="folder-picker-option-archive"]').trigger('click');
    await wrapper.find('[data-testid="confirm-move-note"]').trigger('click');

    expect(mockGatewayExecute).toHaveBeenCalledWith('maenifold.memory', 'MoveMemory', {
      source: 'memory://research/editor',
      destination: 'archive/editor.md',
    });
  });

  it('replaces an active moved tab with the destination URI', async () => {
    const store = useKnowledgeStore();
    store.folders = [{
      name: 'archive',
      path: 'archive',
      fileCount: 0,
      open: false,
      loaded: true,
      loading: false,
      error: null,
      children: [],
      files: [],
    }];
    store.openTabs = ['memory://research/editor'];
    store.activeTabUri = 'memory://research/editor';
    mockGatewayExecute.mockReset();
    mockGatewayExecute
      .mockResolvedValueOnce('Moved memory FILE: memory://research/editor → memory://archive/editor')
      .mockResolvedValueOnce('### Folders\n\n### Files')
      .mockResolvedValueOnce({
        success: true,
        data: {
          uri: 'memory://archive/editor',
          title: 'editor.md',
          content: 'Connected to [[knowledge-crud]].',
          checksum: 'checksum-1',
        },
      });

    const wrapper = mount(KnowledgeRowActions, {
      props: { file: file() },
      global: { stubs: { ...dialogStubs, ...dropdownStubs, ...selectStubs } },
    });

    await wrapper.find('[data-testid="row-action-move"]').trigger('click');
    await wrapper.find('[data-testid="folder-picker-option-archive"]').trigger('click');
    await wrapper.find('[data-testid="confirm-move-note"]').trigger('click');
    await flushPromises();

    expect(store.activeTabUri).toBe('memory://archive/editor');
    expect(store.openTabs).toContain('memory://archive/editor');
    expect(store.openTabs).not.toContain('memory://research/editor');
  });
});
