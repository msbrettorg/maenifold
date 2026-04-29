// @vitest-environment jsdom
import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ref } from 'vue';
import KnowledgeNewNoteDialog from '../KnowledgeNewNoteDialog.vue';

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
  DialogTrigger: { template: '<div><slot /></div>' },
};

describe('KnowledgeNewNoteDialog', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockCallTool.mockReset();
  });

  it('blocks submit until title and WikiLink content are present', async () => {
    const wrapper = mount(KnowledgeNewNoteDialog, {
      global: { stubs: dialogStubs },
    });

    const submit = () => wrapper.find('[data-testid="create-note-submit"]');
    expect(submit().attributes('disabled')).toBeDefined();

    await wrapper.find('[data-testid="new-note-title"]').setValue('Research note');
    await wrapper.find('[data-testid="new-note-content"]').setValue('No graph link yet.');
    expect(submit().attributes('disabled')).toBeDefined();

    await wrapper.find('[data-testid="new-note-content"]').setValue('Connected to [[knowledge-crud]].');
    expect(submit().attributes('disabled')).toBeUndefined();
  });

  it('submits valid notes through WriteMemory and opens the returned note', async () => {
    mockCallTool
      .mockResolvedValueOnce({ success: true, data: { uri: 'memory://research/Research note' } })
      .mockResolvedValueOnce('### Folders\n\n### Files')
      .mockResolvedValueOnce({
        success: true,
        data: {
          uri: 'memory://research/Research note',
          title: 'Research note',
          content: 'Connected to [[knowledge-crud]].',
          checksum: 'checksum-1',
        },
      });

    const wrapper = mount(KnowledgeNewNoteDialog, {
      global: { stubs: dialogStubs },
    });

    await wrapper.find('[data-testid="new-note-title"]').setValue('Research note');
    await wrapper.find('[data-testid="new-note-folder"]').setValue('research');
    await wrapper.find('[data-testid="new-note-content"]').setValue('Connected to [[knowledge-crud]].');
    await wrapper.find('[data-testid="new-note-form"]').trigger('submit');

    expect(mockCallTool).toHaveBeenCalledWith('WriteMemory', {
      title: 'Research note',
      folder: 'research',
      content: 'Connected to [[knowledge-crud]].',
    });
  });
});
