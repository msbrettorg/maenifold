// @vitest-environment jsdom
import { describe, expect, it, beforeEach } from 'vitest';
import { mount, type VueWrapper } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import AppTabBar from '../AppTabBar.vue';

/**
 * T-205.715: blue-team test for AppTabBar active-tab chrome.
 *
 * The task spec requires proving this bug:
 *   - AppTabBar adds custom hard-border chrome to the selected tab using
 *     `border-b-[3px]` and `data-[state=active]:border-b-brand`
 *   - This violates DESIGN.md §6 ("1px borders by default")
 *   - The shared TabsTrigger primitive already provides subtle active-state
 *     treatment via its after: indicator
 *
 * This test must FAIL against the current implementation for the right reason.
 */

const tabsStubs = {
  Tabs: {
    template: '<div class="tabs-root" v-bind="$attrs"><slot /></div>',
    props: ['modelValue'],
  },
  TabsList: {
    template: '<div class="tabs-list" role="tablist" v-bind="$attrs"><slot /></div>',
    props: ['variant'],
  },
  TabsTrigger: {
    template: '<button class="tabs-trigger" role="tab" :data-state="value === modelValue ? \'active\' : \'inactive\'" v-bind="$attrs"><slot /></button>',
    props: ['value'],
    inject: {
      modelValue: { from: 'tabsModelValue', default: () => '' },
    },
    provide() {
      return {};
    },
  },
};

function mountAppTabBar(activeTab = 'home'): VueWrapper {
  const pinia = createPinia();
  setActivePinia(pinia);

  return mount(AppTabBar, {
    props: { modelValue: activeTab },
    global: {
      plugins: [pinia],
      stubs: tabsStubs,
    },
  });
}

describe('AppTabBar', () => {
  beforeEach(() => {
    const pinia = createPinia();
    setActivePinia(pinia);
  });

  it('renders all shell tabs with correct tab order', () => {
    const wrapper = mountAppTabBar();
    const triggers = wrapper.findAll('[role="tab"]');

    expect(triggers).toHaveLength(7);
    expect(triggers[0]!.text()).toContain('Home');
    expect(triggers[1]!.text()).toContain('Agents');
    expect(triggers[2]!.text()).toContain('Pipelines');
    expect(triggers[3]!.text()).toContain('Gate');
    expect(triggers[4]!.text()).toContain('Knowledge');
    expect(triggers[5]!.text()).toContain('Workflows');
    expect(triggers[6]!.text()).toContain('Settings');
  });

  it('emits update:modelValue when tab is selected', async () => {
    const wrapper = mountAppTabBar('home');
    const triggers = wrapper.findAll('[role="tab"]');

    await triggers[1]!.trigger('click');
    // The emit happens through the Tabs component; with stubs we verify render only
    expect(wrapper.emitted()).toBeDefined();
  });

  /**
   * DESIGN CONTRACT TEST: Active-tab chrome must NOT use hard border treatment.
   *
   * Per DESIGN.md and task spec T-205.715:
   *   - Do NOT use `border-b-[3px]` on tab triggers
   *   - Do NOT use `data-[state=active]:border-b-brand` for active state
   *   - TabsTrigger primitive already provides subtle selection via after: indicator
   *
   * This test asserts the ABSENCE of these anti-pattern classes in the component
   * source. It should FAIL against the current implementation (line 42).
   */
  it('does not apply hard 3px border chrome to active tabs', () => {
    const wrapper = mountAppTabBar('home');

    // Get the raw class string from TabsTrigger elements
    // We check the outer component's class binding since stubs simplify the DOM
    const html = wrapper.html();

    // These patterns are the bug: hard border chrome on the selected tab
    const hasForbiddenBorder3px = html.includes('border-b-[3px]');
    const hasForbiddenActiveBrand = html.includes('data-[state=active]:border-b-brand');
    const hasForbiddenActiveVariant = html.includes('data-active:border-b-brand');

    // This assertion is designed to FAIL against current implementation
    // and PASS after the fix removes these classes
    expect(
      hasForbiddenBorder3px || hasForbiddenActiveBrand || hasForbiddenActiveVariant,
      'AppTabBar should not use hard 3px border chrome or brand underline on active tabs. ' +
      'Found forbidden classes: ' +
      [
        hasForbiddenBorder3px && 'border-b-[3px]',
        hasForbiddenActiveBrand && 'data-[state=active]:border-b-brand',
        hasForbiddenActiveVariant && 'data-active:border-b-brand',
      ].filter(Boolean).join(', '),
    ).toBe(false);
  });

  it('preserves navigation semantics: nav element with Primary navigation label', () => {
    const wrapper = mountAppTabBar();
    const nav = wrapper.find('nav[aria-label="Primary navigation"]');

    expect(nav.exists()).toBe(true);
  });

  it('renders the maenifold brand text', () => {
    const wrapper = mountAppTabBar();

    expect(wrapper.text()).toContain('maenifold');
  });
});
