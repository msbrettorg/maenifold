// @vitest-environment jsdom
import { flushPromises, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { reactive } from 'vue';
import { describe, expect, it, vi } from 'vitest';
import SessionModeSwitcher from '../SessionModeSwitcher.vue';
import { createEmptySessionState, type SessionMode, type SessionState } from '../../lib/types';
import { useSessionStore } from '../../stores/session';

const selectStubs = {
  Select: {
    name: 'Select',
    props: ['modelValue', 'disabled'],
    emits: ['update:modelValue'],
    template: '<div data-test="mode-select" :data-model-value="modelValue" :data-disabled="String(disabled)"><slot /></div>',
  },
  SelectTrigger: {
    template: '<button type="button" data-test="mode-trigger" v-bind="$attrs"><slot /></button>',
  },
  SelectContent: { template: '<div><slot /></div>' },
  SelectItem: {
    props: ['value'],
    template: '<div role="option" :data-value="value"><slot /></div>',
  },
};

function mountSwitcher(modes: SessionMode[], currentModeId: string | null = 'ask') {
  const pinia = createPinia();
  setActivePinia(pinia);
  const store = useSessionStore();
  const session = reactive(createEmptySessionState('local-1', 'Architect', '/repo'));
  session.session = {
    id: 'local-1',
    agentName: 'Architect',
    sessionId: 'remote-1',
    title: 'Architect Session',
    lastUpdated: Date.now(),
    cwd: '/repo',
  };
  session.isConnected = true;
  session.currentModeId = currentModeId;
  (session as SessionState & { modes?: SessionMode[] }).modes = modes;
  store.sessions.set(session.sessionId, session);
  store.sessions = new Map(store.sessions);
  store.activeSessionId = session.sessionId;
  const wrapper = mount(SessionModeSwitcher, { global: { plugins: [pinia], stubs: selectStubs } });
  return { wrapper, store, session };
}

describe('SessionModeSwitcher', () => {
  it('renders advertised session modes and marks the current mode selected', () => {
    const { wrapper } = mountSwitcher([
      { id: 'ask', name: 'Ask', description: 'Request permission before edits' },
      { id: 'plan', name: 'Plan only', description: 'Read-only planning' },
    ]);

    expect(wrapper.find('[data-test="session-mode-switcher"]').exists()).toBe(true);
    expect(wrapper.get('[data-test="mode-select"]').attributes('data-model-value')).toBe('ask');
    expect(wrapper.get('[data-test="mode-trigger"]').attributes('aria-label')).toBe('Session mode');
    expect(wrapper.text()).toContain('Ask');
    expect(wrapper.text()).toContain('Plan only');
  });

  it('dispatches mode changes through the session store', async () => {
    const { wrapper, store, session } = mountSwitcher([
      { id: 'ask', name: 'Ask' },
      { id: 'plan', name: 'Plan only' },
    ]);
    const setSessionMode = vi.spyOn(store, 'setSessionMode').mockImplementation(async (modeId: string) => {
      session.currentModeId = modeId;
    });

    wrapper.getComponent({ name: 'Select' }).vm.$emit('update:modelValue', 'plan');
    await flushPromises();

    expect(setSessionMode).toHaveBeenCalledWith('plan');
    expect(session.currentModeId).toBe('plan');
  });

  it('hides when fewer than two modes are advertised', () => {
    expect(mountSwitcher([]).wrapper.find('[data-test="session-mode-switcher"]').exists()).toBe(false);
    expect(mountSwitcher([{ id: 'ask', name: 'Ask' }]).wrapper.find('[data-test="session-mode-switcher"]').exists()).toBe(false);
  });
});
