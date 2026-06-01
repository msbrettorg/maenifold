/* @vitest-environment jsdom */

import { afterEach, describe, expect, it } from 'vitest';
import { mount, type VueWrapper } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';

import SettingsView from './SettingsView.vue';

type AreaId = 'embeddings' | 'appSettings' | 'agents' | 'mcpServers';

function mountSettingsView() {
  const pinia = createPinia();
  setActivePinia(pinia);

  return mount(SettingsView, {
    global: {
      plugins: [pinia],
      stubs: {
        SettingsAppSettings: {
          template: '<div class="app-settings-stub">Stubbed App Settings</div>',
        },
      },
    },
  });
}

function railButton(wrapper: VueWrapper, area: AreaId) {
  return wrapper.get(`#settings-tab-${area}`);
}

function expectPressedStates(wrapper: VueWrapper, activeArea: AreaId) {
  const areas: AreaId[] = ['embeddings', 'appSettings', 'agents', 'mcpServers'];

  for (const area of areas) {
    expect(railButton(wrapper, area).attributes('aria-pressed')).toBe(
      area === activeArea ? 'true' : 'false'
    );
  }
}

afterEach(() => {
  localStorage.clear();
});

describe('SettingsView', () => {
  it('renders all four left-rail entries', () => {
    const wrapper = mountSettingsView();

    expect(
      wrapper
        .findAll('.rail-item-label')
        .map((node: { text: () => string }) => node.text())
    ).toEqual([
      'Embeddings',
      'App Settings',
      'Agents',
      'MCP Servers',
    ]);
  });

  it('defaults the active area to embeddings', () => {
    const wrapper = mountSettingsView();

    expectPressedStates(wrapper, 'embeddings');
  });

  it('switches the active area when a rail entry is clicked', async () => {
    const wrapper = mountSettingsView();

    await railButton(wrapper, 'appSettings').trigger('click');

    expectPressedStates(wrapper, 'appSettings');
  });

  it('shows the correct placeholder content for each non-app-settings area', async () => {
    const wrapper = mountSettingsView();

    const expectations: Array<{
      area: AreaId;
      title: string;
      placeholder: string;
      milestone: string;
    }> = [
      {
        area: 'embeddings',
        title: 'Embeddings',
        placeholder: 'Embeddings tab coming in T-413.3',
        milestone: 'T-413.3',
      },
      {
        area: 'agents',
        title: 'Agents',
        placeholder: 'Agents tab coming in T-413.5',
        milestone: 'T-413.5',
      },
      {
        area: 'mcpServers',
        title: 'MCP Servers',
        placeholder: 'MCP Servers placeholder',
        milestone: 'Deferred',
      },
    ];

    for (const { area, title, placeholder, milestone } of expectations) {
      await railButton(wrapper, area).trigger('click');

      expect(wrapper.get('.panel-title').text()).toBe(title);
      expect(wrapper.get('.placeholder-title').text()).toBe(placeholder);
      expect(wrapper.get('.panel-chip').text()).toBe(milestone);
    }
  });

  it('renders the stubbed app settings component and removes the placeholder', async () => {
    const wrapper = mountSettingsView();

    await railButton(wrapper, 'appSettings').trigger('click');

    expect(wrapper.get('.panel-title').text()).toBe('App Settings');
    expect(wrapper.get('.app-settings-stub').text()).toBe('Stubbed App Settings');
    expect(wrapper.findAll('.placeholder-card')).toHaveLength(0);
  });

  it('renders the hero card with the settings icon, title, and description', () => {
    const wrapper = mountSettingsView();
    const hero = wrapper.get('.settings-hero');

    expect(hero.findAll('.hero-icon-wrap svg')).toHaveLength(1);
    expect(hero.get('.hero-title').text()).toBe('Settings');
    expect(hero.get('.hero-description').text()).toBe(
      'Manage embeddings, app settings, ACP agents, and MCP servers.'
    );
  });

  it('does not render ARIA tab roles', () => {
    const wrapper = mountSettingsView();
    const html = wrapper.html();

    expect(html.match(/role="tab"/g) ?? []).toHaveLength(0);
    expect(html.match(/role="tablist"/g) ?? []).toHaveLength(0);
    expect(html.match(/role="tabpanel"/g) ?? []).toHaveLength(0);
  });
});
