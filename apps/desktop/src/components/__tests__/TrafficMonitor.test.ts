// @vitest-environment jsdom
import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { describe, expect, it } from 'vitest';
import TrafficMonitor from '../TrafficMonitor.vue';

function mountTrafficMonitor() {
  const pinia = createPinia();
  setActivePinia(pinia);

  return mount(TrafficMonitor, {
    global: {
      plugins: [pinia],
      stubs: {
        Badge: { template: '<div v-bind="$attrs"><slot /></div>' },
        Button: { template: '<button v-bind="$attrs"><slot /></button>' },
        Input: { template: '<input v-bind="$attrs" />' },
        ScrollArea: { template: '<div v-bind="$attrs"><slot /></div>' },
        TrafficEntry: { template: '<div />' },
      },
    },
  });
}

describe('TrafficMonitor', () => {
  it('header row wraps chip groups under narrow width (T-304.8)', () => {
    const wrapper = mountTrafficMonitor();
    const title = wrapper.findAll('span').find((node) => node.text() === 'ACP traffic');

    expect(title, 'TrafficMonitor header title should render').toBeDefined();
    const header = title!.element.parentElement as HTMLElement;

    expect(header.classList.contains('flex-wrap')).toBe(true);
  });
});
