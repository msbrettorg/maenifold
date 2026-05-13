// @vitest-environment jsdom
import { flushPromises, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ref } from 'vue';
import SettingsView from '../SettingsView.vue';

const guard = vi.hoisted(() => ({
  markDirty: vi.fn(),
  markClean: vi.fn(),
  confirmIfDirty: vi.fn(async () => true),
  attachBeforeUnload: vi.fn(() => vi.fn()),
}));

const writeConfig = vi.fn(async (content: string) => ({ path: '/maenifold.json', content }));

vi.mock('json-editor-vue', () => ({
  default: {
    props: ['modelValue'],
    emits: ['update:modelValue'],
    template: '<textarea data-testid="settings-json" :value="modelValue" @input="$emit(\'update:modelValue\', $event.target.value)" />',
  },
}));

vi.mock('vanilla-jsoneditor', () => ({
  Mode: { text: 'text' },
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

describe('SettingsView unsaved changes guard', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    writeConfig.mockClear();
    guard.markDirty.mockClear();
    guard.markClean.mockClear();
    guard.confirmIfDirty.mockClear();
    guard.attachBeforeUnload.mockClear();
    guard.confirmIfDirty.mockResolvedValue(true);
    Object.assign(window, {
      electronAPI: {
        configRead: vi.fn(async () => ({
          path: '/maenifold.json',
          exists: true,
          content: '{"agents":[]}',
        })),
        configWrite: writeConfig,
        configPath: vi.fn(async () => '/maenifold.json'),
        onConfigChanged: vi.fn(() => vi.fn()),
      },
    });
  });

  it('uses the shared guard for JSON edits, save success, and revert', async () => {
    const wrapper = mount(SettingsView, {
      global: { stubs: { SettingsEmbeddings: true } },
    });
    await flushPromises();

    await wrapper.get('[data-testid="settings-json"]').setValue('{"agents":[],"settings":{}}');
    expect(guard.markDirty).toHaveBeenCalled();

    const saveButton = wrapper.findAll('button').find((button) => button.text().includes('Save'));
    expect(saveButton).toBeDefined();
    await saveButton!.trigger('click');
    await flushPromises();
    expect(writeConfig).toHaveBeenCalledWith('{"agents":[],"settings":{}}');
    expect(guard.markClean).toHaveBeenCalled();

    await wrapper.get('[data-testid="settings-json"]').setValue('{"agents":[{"name":"draft"}]}');
    const revertButton = wrapper.findAll('button').find((button) => button.text().includes('Revert'));
    expect(revertButton).toBeDefined();
    await revertButton!.trigger('click');
    expect(guard.confirmIfDirty).toHaveBeenCalled();
  });
});
