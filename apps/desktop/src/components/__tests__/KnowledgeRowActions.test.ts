// @vitest-environment jsdom
import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ref } from 'vue';
import KnowledgeRowActions from '../KnowledgeRowActions.vue';
import type { KnowledgeFileEntry } from '../../stores/knowledge';

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
    mockCallTool.mockReset();
    mockCallTool.mockResolvedValue('### Folders\n\n### Files');
  });

  it('confirms delete with the note name and calls DeleteMemory with confirm true', async () => {
    const wrapper = mount(KnowledgeRowActions, {
      props: { file: file() },
      global: { stubs: { ...dialogStubs, ...dropdownStubs } },
    });

    await wrapper.find('[data-testid="row-action-delete"]').trigger('click');
    expect(wrapper.text()).toContain('This will permanently delete editor.md.');

    await wrapper.find('[data-testid="confirm-delete-note"]').trigger('click');
    expect(mockCallTool).toHaveBeenCalledWith('DeleteMemory', {
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

    expect(mockCallTool).toHaveBeenCalledWith('MoveMemory', {
      source: 'memory://research/editor',
      destination: 'research/renamed.md',
    });
  });

  it('moves notes with MoveMemory into the selected folder', async () => {
    const wrapper = mount(KnowledgeRowActions, {
      props: { file: file() },
      global: { stubs: { ...dialogStubs, ...dropdownStubs } },
    });

    await wrapper.find('[data-testid="row-action-move"]').trigger('click');
    await wrapper.find('[data-testid="move-note-folder"]').setValue('archive');
    await wrapper.find('[data-testid="confirm-move-note"]').trigger('click');

    expect(mockCallTool).toHaveBeenCalledWith('MoveMemory', {
      source: 'memory://research/editor',
      destination: 'archive/editor.md',
    });
  });
});
