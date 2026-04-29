// @vitest-environment jsdom
import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import FolderPicker from '../FolderPicker.vue';

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
    emits: ['select'],
    template: '<button type="button" :data-testid="`folder-picker-option-${value}`" @click="$parent.$emit(\'update:modelValue\', value)"><slot /></button>',
  },
};

describe('FolderPicker', () => {
  it('renders available folders and emits selected paths', async () => {
    const wrapper = mount(FolderPicker, {
      props: {
        modelValue: '',
        availableFolders: ['research', 'archive'],
        placeholder: 'Choose folder',
      },
      global: { stubs: selectStubs },
    });

    expect(wrapper.text()).toContain('research');
    expect(wrapper.text()).toContain('archive');

    await wrapper.find('[data-testid="folder-picker-option-archive"]').trigger('click');
    expect(wrapper.emitted('update:modelValue')).toEqual([['archive']]);
  });

  it('emits normalized new folder paths after validation', async () => {
    const wrapper = mount(FolderPicker, {
      props: {
        modelValue: '',
        availableFolders: ['research'],
      },
      global: { stubs: selectStubs },
    });

    await wrapper.find('[data-testid="folder-picker-option-__create_new_folder__"]').trigger('click');
    await wrapper.find('[data-testid="folder-picker-new-input"]').setValue('new/folder');

    expect(wrapper.text()).toContain('Will be created at: new/folder');
    await wrapper.find('[data-testid="folder-picker-use-new"]').trigger('click');
    expect(wrapper.emitted('update:modelValue')).toEqual([['new/folder']]);
  });
});
