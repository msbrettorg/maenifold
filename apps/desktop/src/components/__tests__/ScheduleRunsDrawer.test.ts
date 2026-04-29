// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import ScheduleRunsDrawer from '../ScheduleRunsDrawer.vue';

const readResource = vi.fn();
const subscribeResource = vi.fn();
const callTool = vi.fn();
const connect = vi.fn();
const connected = { value: true };

vi.mock('@/composables/useMcpResources', () => ({
  useMcpResources: () => ({
    readResource,
    subscribeResource,
  }),
}));

vi.mock('@/composables/useMcpClient', () => ({
  useMcpClient: () => ({
    callTool,
    connect,
    connected,
  }),
}));

function mountDrawer() {
  return mount(ScheduleRunsDrawer, {
    props: {
      open: true,
      scheduleId: 'daily-summary',
    },
    attachTo: document.body,
  });
}

describe('ScheduleRunsDrawer', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
    readResource.mockReset();
    subscribeResource.mockReset();
    callTool.mockReset();
    connect.mockReset();
    connected.value = true;
    subscribeResource.mockResolvedValue(vi.fn());
    callTool.mockResolvedValue({ message: 'ran' });
  });

  it('renders empty state', async () => {
    readResource.mockResolvedValue({ uri: 'schedule://logs/daily-summary', text: '[]' });

    const wrapper = mountDrawer();
    await flushPromises();

    expect(document.body.textContent).toContain('No runs recorded yet.');
    wrapper.unmount();
  });

  it('renders multiple runs', async () => {
    readResource.mockResolvedValue({
      uri: 'schedule://logs/daily-summary',
      text: JSON.stringify([
        { fileName: '20260429T140000Z.log', content: 'ok' },
        { fileName: '20260429T150000Z.log', content: 'exit code: 1' },
      ]),
    });

    const wrapper = mountDrawer();
    await flushPromises();

    expect(document.body.textContent).toContain('success');
    expect(document.body.textContent).toContain('failed');
    wrapper.unmount();
  });

  it('shows failed badge and retry button', async () => {
    readResource.mockResolvedValue({
      uri: 'schedule://logs/daily-summary',
      text: JSON.stringify([{ fileName: '20260429T150000Z.log', content: 'exit code: 1' }]),
    });

    const wrapper = mountDrawer();
    await flushPromises();
    const retryButton = Array.from(document.body.querySelectorAll('button'))
      .find((button) => button.textContent?.includes('Retry now'));
    retryButton?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    await flushPromises();

    expect(document.body.textContent).toContain('failed');
    expect(document.body.textContent).toContain('Retry now');
    expect(callTool).toHaveBeenCalledWith('Schedule', { action: 'run_now', id: 'daily-summary' });
    wrapper.unmount();
  });

  it('expands run log content', async () => {
    readResource.mockResolvedValue({
      uri: 'schedule://logs/daily-summary',
      text: JSON.stringify([{ fileName: '20260429T140000Z.log', content: 'hello from log' }]),
    });

    const wrapper = mountDrawer();
    await flushPromises();
    document.body.querySelector('tbody tr')?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    await flushPromises();

    expect(document.body.textContent).toContain('stdout');
    expect(document.body.textContent).toContain('hello from log');
    wrapper.unmount();
  });
});
