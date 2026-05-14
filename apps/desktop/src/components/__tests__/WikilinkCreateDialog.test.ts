// @vitest-environment jsdom
import { mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import WikilinkCreateDialog from '../WikilinkCreateDialog.vue';

const writeMemory = vi.hoisted(() => vi.fn());
vi.mock('../../composables/useMemoryApi', () => ({
  useMemoryApi: () => ({ writeMemory }),
}));

const stubs = {
  Dialog: { props: ['open'], template: '<div v-if="open"><slot /></div>' },
  DialogContent: { template: '<section><slot /></section>' },
  DialogDescription: { template: '<p><slot /></p>' },
  DialogFooter: { template: '<footer><slot /></footer>' },
  DialogHeader: { template: '<header><slot /></header>' },
  DialogTitle: { template: '<h2><slot /></h2>' },
  FolderPicker: {
    props: ['modelValue', 'availableFolders'],
    emits: ['update:modelValue'],
    template: '<div data-testid="folder-picker"><button type="button" @click="$emit(\'update:modelValue\', \'research\')">research</button></div>',
  },
};

function mountDialog() {
  return mount(WikilinkCreateDialog, {
    props: { open: true, concept: 'Unresolved Concept', availableFolders: ['research'] },
    global: { stubs },
  });
}

describe('WikilinkCreateDialog', () => {
  beforeEach(() => {
    writeMemory.mockReset();
  });

  it('renders the unresolved concept title and folder picker', () => {
    const wrapper = mountDialog();

    expect(wrapper.text()).toContain('Create note for “Unresolved Concept”?');
    expect(wrapper.find('[data-testid="folder-picker"]').exists()).toBe(true);
  });

  it('clicking Create with the root folder writes a stub note', async () => {
    writeMemory.mockResolvedValueOnce('memory://Unresolved Concept');
    const wrapper = mountDialog();

    await wrapper.find('[data-testid="wikilink-create-submit"]').trigger('click');

    expect(writeMemory).toHaveBeenCalledWith({
      title: 'Unresolved Concept',
      content: '[[Unresolved Concept]]',
      folder: undefined,
    });
    expect(wrapper.emitted('created')).toEqual([['memory://Unresolved Concept']]);
  });

  it('clicking Cancel emits cancel and does not write a note', async () => {
    const wrapper = mountDialog();

    await wrapper.find('[data-testid="wikilink-create-cancel"]').trigger('click');

    expect(wrapper.emitted('cancel')).toHaveLength(1);
    expect(writeMemory).not.toHaveBeenCalled();
  });

  it('renders submit errors from WriteMemory', async () => {
    writeMemory.mockRejectedValueOnce(new Error('write unavailable'));
    const wrapper = mountDialog();

    await wrapper.find('[data-testid="wikilink-create-submit"]').trigger('click');

    expect(wrapper.text()).toContain('write unavailable');
  });
});
