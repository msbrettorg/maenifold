// @vitest-environment jsdom
import { describe, expect, it, vi } from 'vitest';
import { mount } from '@vue/test-utils';
import { nextTick } from 'vue';
import { createPinia, setActivePinia, type Pinia } from 'pinia';
import GlobalCommandPalette from '../GlobalCommandPalette.vue';
import { useAppShellStore } from '../../stores/app-shell';

function createTestingPinia(): Pinia {
  const pinia = createPinia();
  setActivePinia(pinia);
  return pinia;
}

function mountPalette() {
  const pinia = createTestingPinia();
  return mount(GlobalCommandPalette, {
    global: {
      plugins: [pinia],
      stubs: commandStubs,
    },
  });
}

const commandStubs = {
  CommandDialog: { props: ['open'], template: '<div v-if="open"><slot /></div>' },
  CommandInput: { template: '<input />' },
  CommandList: { template: '<div><slot /></div>' },
  CommandEmpty: { template: '<div><slot /></div>' },
  CommandGroup: { props: ['heading'], template: '<section><h2>{{ heading }}</h2><slot /></section>' },
  CommandItem: {
    props: ['value'],
    emits: ['select'],
    template: '<div role="option" :data-value="value" @click="$emit(\'select\')"><slot /></div>',
  },
  CommandSeparator: { template: '<hr />' },
};

describe('GlobalCommandPalette', () => {
  it('renders when commandPaletteOpen is true', async () => {
    const wrapper = mountPalette();
    useAppShellStore().commandPaletteOpen = true;
    await nextTick();

    expect(wrapper.text()).toContain('Tabs');
    expect(wrapper.text()).toContain('Recent Sessions');
  });

  it('shows tab items and selects a tab', async () => {
    const wrapper = mountPalette();
    const appShellStore = useAppShellStore();
    appShellStore.commandPaletteOpen = true;
    const setActiveTab = vi.spyOn(appShellStore, 'setActiveTab');
    await nextTick();

    await wrapper.find('[data-value="tab Gate gate"]').trigger('click');

    expect(setActiveTab).toHaveBeenCalledWith('gate');
    expect(appShellStore.activeTab).toBe('gate');
  });

  it('closes on selection', async () => {
    const wrapper = mountPalette();
    const appShellStore = useAppShellStore();
    appShellStore.commandPaletteOpen = true;
    await nextTick();

    await wrapper.find('[data-value="tab Home home"]').trigger('click');

    expect(appShellStore.commandPaletteOpen).toBe(false);
  });
});
