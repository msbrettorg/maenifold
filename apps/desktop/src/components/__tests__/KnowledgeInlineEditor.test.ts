// @vitest-environment jsdom
import { flushPromises, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ref } from 'vue';
import KnowledgeInlineEditor from '../KnowledgeInlineEditor.vue';
import type { MemoryFile } from '../../lib/workflow-types';

const mockCallTool = vi.fn();
const guard = vi.hoisted(() => ({
  markDirty: vi.fn(),
  markClean: vi.fn(),
  confirmIfDirty: vi.fn(async () => true),
  attachBeforeUnload: vi.fn(() => vi.fn()),
}));

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

vi.mock('../../composables/useUnsavedChangesGuard', () => ({
  useUnsavedChangesGuard: vi.fn(() => ({
    isDirty: ref(false),
    markDirty: guard.markDirty,
    markClean: guard.markClean,
    confirmIfDirty: guard.confirmIfDirty,
    attachBeforeUnload: guard.attachBeforeUnload,
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

function note(): MemoryFile {
  return {
    uri: 'memory://research/editor',
    title: 'Editor note',
    content: 'Original [[knowledge-crud]] content.',
    checksum: 'checksum-1',
    created: null,
    modified: null,
  };
}

describe('KnowledgeInlineEditor', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockCallTool.mockReset();
    guard.markDirty.mockClear();
    guard.markClean.mockClear();
    guard.confirmIfDirty.mockClear();
    guard.attachBeforeUnload.mockClear();
    guard.confirmIfDirty.mockResolvedValue(true);
  });

  it('saves markdown through EditMemory with the last-read checksum', async () => {
    mockCallTool
      .mockResolvedValueOnce({ success: true, data: { uri: 'memory://research/editor' } })
      .mockResolvedValueOnce({
        success: true,
        data: {
          ...note(),
          content: 'Updated [[knowledge-crud]] content.',
          checksum: 'checksum-2',
        },
      });

    const wrapper = mount(KnowledgeInlineEditor, {
      props: { note: note() },
      global: { stubs: dialogStubs },
    });

    await wrapper.find('[data-testid="inline-editor-content"]').setValue('Updated [[knowledge-crud]] content.');
    await wrapper.find('[data-testid="inline-editor-save"]').trigger('click');

    expect(mockCallTool).toHaveBeenCalledWith('EditMemory', {
      identifier: 'memory://research/editor',
      operation: 'find_replace',
      findText: 'Original [[knowledge-crud]] content.',
      expectedCount: 1,
      content: 'Updated [[knowledge-crud]] content.',
      checksum: 'checksum-1',
    });
  });

  it('shows a conflict dialog when the checksum is stale', async () => {
    mockCallTool.mockResolvedValueOnce('ERROR: Checksum mismatch. Expected: checksum-1, Actual: checksum-2');

    const wrapper = mount(KnowledgeInlineEditor, {
      props: { note: note() },
      global: { stubs: dialogStubs },
    });

    await wrapper.find('[data-testid="inline-editor-content"]').setValue('Conflicting [[knowledge-crud]] content.');
    await wrapper.find('[data-testid="inline-editor-save"]').trigger('click');

    expect(wrapper.text()).toContain('Edit conflict');
    expect(wrapper.text()).toContain('Discard mine and reload');
    expect(wrapper.text()).toContain('Force overwrite');
  });

  it('uses the shared unsaved changes guard for edit, save, and cancel', async () => {
    mockCallTool
      .mockResolvedValueOnce({ success: true, data: { uri: 'memory://research/editor' } })
      .mockResolvedValueOnce({
        success: true,
        data: {
          ...note(),
          content: 'Guarded [[knowledge-crud]] content.',
          checksum: 'checksum-2',
        },
      });

    const wrapper = mount(KnowledgeInlineEditor, {
      props: { note: note() },
      global: { stubs: dialogStubs },
    });

    await wrapper.find('[data-testid="inline-editor-content"]').setValue('Guarded [[knowledge-crud]] content.');
    expect(guard.markDirty).toHaveBeenCalled();

    await wrapper.find('[data-testid="inline-editor-save"]').trigger('click');
    await flushPromises();
    expect(guard.markClean).toHaveBeenCalled();

    await wrapper.find('[data-testid="inline-editor-content"]').setValue('Dirty again [[knowledge-crud]].');
    await wrapper.get('button').trigger('click');
    expect(guard.confirmIfDirty).toHaveBeenCalled();
  });
});
