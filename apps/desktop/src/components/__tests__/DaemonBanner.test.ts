// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { flushPromises, mount } from '@vue/test-utils';
import DaemonBanner from '../DaemonBanner.vue';
import { useDaemonHealth } from '../../composables/useDaemonHealth';
import { useSessionStore } from '../../stores/session';

const reconnectSession = vi.fn();

vi.mock('../../composables/usePipelineMcp', () => ({
  getSharedMcpClient: () => ({
    reconnectSession,
  }),
}));

const fetchMock = vi.fn();

function okResponse(): Response {
  return { ok: true, status: 200, statusText: 'OK' } as Response;
}

async function setDaemonOnline(online: boolean): Promise<void> {
  fetchMock.mockReset();
  fetchMock.mockResolvedValueOnce(online ? okResponse() : { ok: false, status: 503, statusText: 'Down' });
  await useDaemonHealth().forceCheck();
}

describe('DaemonBanner', () => {
  let pinia: ReturnType<typeof createPinia>;

  beforeEach(async () => {
    pinia = createPinia();
    setActivePinia(pinia);
    reconnectSession.mockReset();
    reconnectSession.mockResolvedValue(undefined);
    vi.stubGlobal('fetch', fetchMock);
    await setDaemonOnline(true);
    useSessionStore().clearConnectionError();
  });

  afterEach(() => {
    useDaemonHealth().stop();
    vi.unstubAllGlobals();
  });

  it('is hidden when daemon is online and connectionError is null', () => {
    const wrapper = mount(DaemonBanner, {
      global: { plugins: [pinia] },
    });

    expect(wrapper.find('[role="alert"]').exists()).toBe(false);
  });

  it('is shown when daemon is offline', async () => {
    await setDaemonOnline(false);

    const wrapper = mount(DaemonBanner, {
      global: { plugins: [pinia] },
    });

    expect(wrapper.find('[role="alert"]').exists()).toBe(true);
    expect(wrapper.text()).toContain('Daemon offline.');
  });

  it('is shown when connectionError is set', () => {
    const store = useSessionStore();
    store.connectionError = 'resume failed';

    const wrapper = mount(DaemonBanner, {
      global: { plugins: [pinia] },
    });

    expect(wrapper.find('[role="alert"]').exists()).toBe(true);
    expect(wrapper.text()).toContain('Connection issue: resume failed');
  });

  it('retry calls forceCheck and reconnectSession', async () => {
    const store = useSessionStore();
    store.connectionError = 'resume failed';
    fetchMock.mockReset();
    fetchMock.mockResolvedValueOnce(okResponse());

    const wrapper = mount(DaemonBanner, {
      global: { plugins: [pinia] },
    });

    const retry = wrapper.findAll('button').find((button) => button.text() === 'Retry');
    expect(retry).toBeDefined();
    await retry!.trigger('click');
    await flushPromises();

    expect(fetchMock).toHaveBeenCalledOnce();
    expect(reconnectSession).toHaveBeenCalledOnce();
  });

  it('toggles inline help from How to start', async () => {
    await setDaemonOnline(false);
    const wrapper = mount(DaemonBanner, {
      global: { plugins: [pinia] },
    });

    expect(wrapper.text()).not.toContain('Start the daemon by running');

    const howToStart = wrapper.findAll('button').find((button) => button.text() === 'How to start');
    expect(howToStart).toBeDefined();
    await howToStart!.trigger('click');

    expect(wrapper.text()).toContain('Start the daemon by running');
    expect(wrapper.text()).toContain('maenifold --http');
  });
});
