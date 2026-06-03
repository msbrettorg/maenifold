/* @vitest-environment jsdom */

import { flushPromises, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { nextTick } from 'vue';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import ChatView from './ChatView.vue';
import ConfigOptionPicker from './ConfigOptionPicker.vue';
import { useSessionStore } from '../stores/session';
import type { SavedSession, SessionConfigOption } from '../lib/types';

const mountedWrappers: Array<ReturnType<typeof mount>> = [];

const baseSession: SavedSession = {
  id: 'saved-session-1',
  agentName: 'Copilot CLI',
  sessionId: 'session-1',
  title: 'ACP chat',
  lastUpdated: Date.now(),
  cwd: 'C:\\src\\maenifold\\worktrees\\T-500.2',
  supportsLoadSession: true,
};

function buildConfigOptions(): SessionConfigOption[] {
  return [
    {
      id: 'allow_all',
      name: 'Allow All',
      description: 'Automatically approve all tool, path, and URL requests.',
      category: 'mode',
      type: 'boolean',
      currentValue: false,
    },
    {
      id: 'model',
      name: 'Model',
      category: 'model',
      type: 'select',
      currentValue: 'gpt-5.2',
      options: [
        { value: 'gpt-5.2', name: 'GPT 5.2' },
        { value: 'gpt-5.5', name: 'GPT 5.5' },
      ],
    },
  ];
}

async function settle() {
  await nextTick();
  await flushPromises();
  await nextTick();
}

function mountChatView() {
  const pinia = createPinia();
  setActivePinia(pinia);

  const sessionStore = useSessionStore();
  sessionStore.currentSession = { ...baseSession };
  sessionStore.configOptions = buildConfigOptions();
  sessionStore.error = null;
  sessionStore.isLoading = false;
  sessionStore.isReconnecting = false;
  sessionStore.availableCommands = [];

  const wrapper = mount(ChatView, {
    global: {
      plugins: [pinia],
    },
  });
  mountedWrappers.push(wrapper);
  return { wrapper, sessionStore };
}

function findPicker(wrapper: ReturnType<typeof mount>, configId: string) {
  const picker = wrapper.findAllComponents(ConfigOptionPicker).find(
    (candidate) => (candidate.props('option') as SessionConfigOption).id === configId
  );
  expect(picker, `Expected picker for ${configId}`).toBeTruthy();
  return picker!;
}

beforeEach(() => {
  vi.restoreAllMocks();
});

afterEach(() => {
  while (mountedWrappers.length > 0) {
    mountedWrappers.pop()?.unmount();
  }
  document.body.innerHTML = '';
});

describe('ChatView config option failures', () => {
  it('surfaces rejected config-option changes via the session error surface and keeps the server value visible', async () => {
    const { wrapper, sessionStore } = mountChatView();
    const setConfigOptionMock = vi
      .spyOn(sessionStore, 'setConfigOption')
      .mockRejectedValue(
        new Error(
          "Failed to set config option 'allow_all' to 'on': Permission service is unavailable for this session."
        )
      );

    expect(wrapper.get('[data-config-id="allow_all"] .boolean-state').text()).toBe('OFF');

    await wrapper.get('[data-config-id="allow_all"] .config-button.boolean').trigger('click');
    await settle();

    expect(setConfigOptionMock).toHaveBeenCalledWith('allow_all', true);
    expect(sessionStore.error).toContain('Permission service is unavailable for this session.');
    expect(wrapper.get('[data-config-id="allow_all"] .boolean-state').text()).toBe('OFF');
  });

  it('still reflects successful config-option updates from the authoritative session snapshot', async () => {
    const { wrapper, sessionStore } = mountChatView();
    vi.spyOn(sessionStore, 'setConfigOption').mockImplementation(async (_configId, value) => {
      sessionStore.configOptions = sessionStore.configOptions.map((option) =>
        option.id === 'model' && option.type === 'select' && typeof value === 'string'
          ? { ...option, currentValue: value }
          : option
      );
    });

    expect(findPicker(wrapper, 'model').get('.config-name').text()).toBe('GPT 5.2');

    await wrapper.get('[data-config-id="model"] .config-button.select').trigger('click');
    await wrapper.get('[data-config-id="model"] .dropdown-item:last-child').trigger('click');
    await settle();

    expect(sessionStore.error).toBeNull();
    expect(findPicker(wrapper, 'model').get('.config-name').text()).toBe('GPT 5.5');
  });
});
