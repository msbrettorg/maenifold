/* @vitest-environment jsdom */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { mount, type VueWrapper } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';

vi.mock('../lib/mcp/client', async (importActual) => {
  const actual = await importActual<typeof import('../lib/mcp/client')>();
  return { ...actual, mcpHttpClient: { callTool: vi.fn() } };
});

import { mcpHttpClient } from '../lib/mcp/client';
import SettingsView from './SettingsView.vue';

type AreaId = 'embeddings' | 'appSettings' | 'agents' | 'mcpServers';

function toolText(value: unknown) {
  return {
    content: [{ type: 'text', text: JSON.stringify(value) }],
  };
}

const callToolMock = vi.mocked(mcpHttpClient.callTool);

function mountSettingsView() {
  const pinia = createPinia();
  setActivePinia(pinia);

  return mount(SettingsView, {
    global: {
      plugins: [pinia],
      stubs: {
        SettingsEmbeddings: {
          template: '<div class="embeddings-stub">Stubbed Embeddings</div>',
        },
        SettingsAppSettings: {
          template: '<div class="app-settings-stub">Stubbed App Settings</div>',
        },
        SettingsAgents: {
          template: '<div class="agents-stub">Stubbed Agents</div>',
        },
        SettingsMcpServers: {
          template: '<div class="mcp-servers-stub">Stubbed MCP Servers</div>',
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

beforeEach(() => {
  callToolMock.mockReset();
  callToolMock.mockResolvedValue(
    toolText({
      agents: {
        'Copilot CLI': {
          command: 'npx.cmd',
          args: ['@github/copilot@latest', '--acp'],
        },
      },
      mcpServers: {
        filesystem: {
          command: 'npx',
          args: ['-y', '@modelcontextprotocol/server-filesystem', '.'],
        },
      },
    })
  );
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
    expect(wrapper.get('.embeddings-stub').text()).toBe('Stubbed Embeddings');
  });

  it('switches the active area when a rail entry is clicked', async () => {
    const wrapper = mountSettingsView();

    await railButton(wrapper, 'appSettings').trigger('click');

    expectPressedStates(wrapper, 'appSettings');
  });

  it('renders each settings area using its dedicated component stub', async () => {
    const wrapper = mountSettingsView();

    const expectations: Array<{
      area: AreaId;
      title: string;
      stubSelector: string;
      stubText: string;
      milestone: string;
    }> = [
      {
        area: 'embeddings',
        title: 'Embeddings',
        stubSelector: '.embeddings-stub',
        stubText: 'Stubbed Embeddings',
        milestone: 'T-413.3',
      },
      {
        area: 'appSettings',
        title: 'App Settings',
        stubSelector: '.app-settings-stub',
        stubText: 'Stubbed App Settings',
        milestone: 'T-413.4',
      },
      {
        area: 'agents',
        title: 'Agents',
        stubSelector: '.agents-stub',
        stubText: 'Stubbed Agents',
        milestone: 'T-413.5',
      },
      {
        area: 'mcpServers',
        title: 'MCP Servers',
        stubSelector: '.mcp-servers-stub',
        stubText: 'Stubbed MCP Servers',
        milestone: 'T-413.5',
      },
    ];

    for (const { area, title, stubSelector, stubText, milestone } of expectations) {
      await railButton(wrapper, area).trigger('click');

      expect(wrapper.get('.panel-title').text()).toBe(title);
      expect(wrapper.get(stubSelector).text()).toBe(stubText);
      expect(wrapper.get('.panel-chip').text()).toBe(milestone);
      expect(wrapper.findAll('.placeholder-card')).toHaveLength(0);
    }
  });

  it('renders the stubbed agents component and removes the placeholder', async () => {
    const wrapper = mountSettingsView();

    await railButton(wrapper, 'agents').trigger('click');

    expect(wrapper.get('.panel-title').text()).toBe('Agents');
    expect(wrapper.get('.agents-stub').text()).toBe('Stubbed Agents');
    expect(wrapper.findAll('.placeholder-card')).toHaveLength(0);
  });

  it('renders the stubbed mcp servers component and removes the placeholder', async () => {
    const wrapper = mountSettingsView();

    await railButton(wrapper, 'mcpServers').trigger('click');

    expect(wrapper.get('.panel-title').text()).toBe('MCP Servers');
    expect(wrapper.get('.mcp-servers-stub').text()).toBe('Stubbed MCP Servers');
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
