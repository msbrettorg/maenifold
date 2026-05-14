// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import ScheduleRunsDrawer from '../ScheduleRunsDrawer.vue';

const readResource = vi.fn();
const subscribeResource = vi.fn();
const callTool = vi.fn();
const connect = vi.fn();
const connected = { value: true };

function deferred<T>() {
  let resolve!: (value: T | PromiseLike<T>) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((promiseResolve, promiseReject) => {
    resolve = promiseResolve;
    reject = promiseReject;
  });
  return { promise, resolve, reject };
}

vi.mock('@/composables/useMcpResources', () => ({ useMcpResources: () => ({ readResource, subscribeResource }) }));
vi.mock('@/composables/useMcpClient', () => ({ useMcpClient: () => ({ callTool, connect, connected }) }));

function mountDrawer() {
  return mount(ScheduleRunsDrawer, {
    props: {
      open: true,
      scheduleId: 'daily-summary',
    },
    attachTo: document.body,
  });
}

function renderedBadges() {
  return Array.from(document.body.querySelectorAll('[data-slot="badge"]')).map((badge) => ({
    label: badge.textContent?.trim(),
    variant: badge.getAttribute('data-variant'),
    className: badge.getAttribute('class') ?? '',
  }));
}

function retryButton() {
  return Array.from(document.body.querySelectorAll('button'))
    .find((button) => button.textContent?.includes('Retry now'));
}

async function expandFirstRun() {
  document.body.querySelector('tbody tr')?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
  await flushPromises();
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
        { fileName: '20260429T140000Z.log', status: 'success', content: 'ok' },
        { fileName: '20260429T150000Z.log', content: 'exit code: 1' },
      ]),
    });

    const wrapper = mountDrawer();
    await flushPromises();

    expect(document.body.textContent).toContain('success');
    expect(document.body.textContent).toContain('failed');
    wrapper.unmount();
  });

  it('does not subscribe after the drawer closes while runs are loading', async () => {
    const pendingRuns = deferred<{ uri: string; text: string }>();
    readResource.mockReturnValue(pendingRuns.promise);

    const wrapper = mountDrawer();
    await wrapper.setProps({ open: false });

    pendingRuns.resolve({ uri: 'schedule://logs/daily-summary', text: '[]' });
    await flushPromises();

    expect(subscribeResource).not.toHaveBeenCalled();
    wrapper.unmount();
  });

  it('renders cancelled, running, and unknown outcomes with explicit badge variants', async () => {
    readResource.mockResolvedValue({
      uri: 'schedule://logs/daily-summary',
      text: JSON.stringify([
        { fileName: '20260429T160000Z.log', status: 'cancelled', content: 'cancelled by user' },
        { fileName: '20260429T170000Z.log', status: 'in-progress', content: 'running' },
        { fileName: '20260429T180000Z.log', status: 'skipped', content: 'not scheduled' },
      ]),
    });

    const wrapper = mountDrawer();
    await flushPromises();
    const outcomes = renderedBadges();

    expect(outcomes).toEqual([
      expect.objectContaining({ label: 'cancelled', variant: 'secondary' }),
      expect.objectContaining({ label: 'running', variant: 'secondary' }),
      expect.objectContaining({ label: 'unknown', variant: 'outline' }),
    ]);
    expect(outcomes[1].className).toContain('animate-pulse');
    expect(document.body.textContent).not.toContain('success');
    wrapper.unmount();
  });

  it('shows failed badge and retry button', async () => {
    readResource.mockResolvedValue({
      uri: 'schedule://logs/daily-summary',
      text: JSON.stringify([{ fileName: '20260429T150000Z.log', content: 'exit code: 1' }]),
    });

    const wrapper = mountDrawer();
    await flushPromises();
    retryButton()?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    await flushPromises();

    expect(document.body.textContent).toContain('failed');
    expect(document.body.textContent).toContain('Retry now');
    expect(callTool).toHaveBeenCalledWith('Schedule', { action: 'run_now', id: 'daily-summary' });
    wrapper.unmount();
  });

  it('guards Retry now against immediate duplicate clicks', async () => {
    const retry = deferred<{ message: string }>();
    readResource.mockResolvedValue({
      uri: 'schedule://logs/daily-summary',
      text: JSON.stringify([{ fileName: '20260429T150000Z.log', content: 'exit code: 1' }]),
    });
    callTool.mockReturnValue(retry.promise);

    const wrapper = mountDrawer();
    await flushPromises();
    const button = retryButton();
    button?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    button?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    await flushPromises();

    expect(callTool).toHaveBeenCalledTimes(1);
    retry.resolve({ message: 'ran' });
    await flushPromises();
    wrapper.unmount();
  });

  it('normalizes cancelled status even when content contains "error"', async () => {
    readResource.mockResolvedValue({
      uri: 'schedule://logs/daily-summary',
      text: JSON.stringify([{ fileName: '20260429T150000Z.log', status: 'cancelled', content: 'cancelled due to error in upstream dep' }]),
    });

    const wrapper = mountDrawer();
    await flushPromises();

    expect(renderedBadges()).toEqual([expect.objectContaining({ label: 'cancelled', variant: 'secondary' })]);
    expect(retryButton()).toBeUndefined();
    wrapper.unmount();
  });

  it('normalizes running status even when content contains "timed out"', async () => {
    readResource.mockResolvedValue({
      uri: 'schedule://logs/daily-summary',
      text: JSON.stringify([{ fileName: '20260429T150000Z.log', status: 'in-progress', content: 'task timed out at 30s' }]),
    });

    const wrapper = mountDrawer();
    await flushPromises();

    expect(renderedBadges()).toEqual([expect.objectContaining({ label: 'running', variant: 'secondary', className: expect.stringContaining('animate-pulse') })]);
    expect(retryButton()).toBeUndefined();
    wrapper.unmount();
  });

  it('normalizes statusless legacy run with error content as failed', async () => {
    readResource.mockResolvedValue({
      uri: 'schedule://logs/daily-summary',
      text: JSON.stringify([{ fileName: '20260429T150000Z.log', content: 'exit code 1' }]),
    });

    const wrapper = mountDrawer();
    await flushPromises();

    expect(renderedBadges()).toEqual([expect.objectContaining({ label: 'failed', variant: 'destructive' })]);
    expect(retryButton()).toBeTruthy();
    wrapper.unmount();
  });

  it('normalizes success status as success even when content contains "error"', async () => {
    readResource.mockResolvedValue({
      uri: 'schedule://logs/daily-summary',
      text: JSON.stringify([{ fileName: '20260429T150000Z.log', status: 'success', content: 'completed but exit code 0 with error logged' }]),
    });

    const wrapper = mountDrawer();
    await flushPromises();

    expect(renderedBadges()).toEqual([expect.objectContaining({ label: 'success', variant: 'default' })]);
    wrapper.unmount();
  });

  it('normalizes skipped status with error content as failed (legacy heuristic)', async () => {
    readResource.mockResolvedValue({
      uri: 'schedule://logs/daily-summary',
      text: JSON.stringify([{ fileName: '20260429T150000Z.log', status: 'skipped', content: 'exit code 1' }]),
    });

    const wrapper = mountDrawer();
    await flushPromises();

    expect(renderedBadges()).toEqual([expect.objectContaining({ label: 'failed', variant: 'destructive' })]);
    wrapper.unmount();
  });

  it('expands run log content', async () => {
    readResource.mockResolvedValue({
      uri: 'schedule://logs/daily-summary',
      text: JSON.stringify([{ fileName: '20260429T140000Z.log', content: 'hello from log' }]),
    });

    const wrapper = mountDrawer();
    await flushPromises();
    await expandFirstRun();

    expect(document.body.textContent).toContain('stdout');
    expect(document.body.textContent).toContain('hello from log');
    wrapper.unmount();
  });

  it('renders MCP action badge and parameters when run.mcp_action is set', async () => {
    readResource.mockResolvedValue({
      uri: 'schedule://logs/daily-summary',
      text: JSON.stringify([{
        fileName: '20260429T140000Z.log', status: 'success', mcp_action: 'maenifold.gate.gate_Status',
        mcp_parameters: { root: '/workspace' }, mcp_result: { ok: true },
      }, { fileName: '20260429T150000Z.log', status: 'success', mcp_action: 'maenifold.gate.gate_Status', mcp_result: null }]),
    });

    const wrapper = mountDrawer();
    await flushPromises();
    await expandFirstRun();

    expect(document.body.textContent).toContain('MCP action:');
    expect(renderedBadges()).toContainEqual(expect.objectContaining({ label: 'maenifold.gate.gate_Status', variant: 'outline' }));
    expect(document.body.textContent).toContain('"root": "/workspace"');
    expect(document.body.textContent).toContain('"ok": true');
    document.body.querySelectorAll('tbody tr.cursor-pointer')[1]?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    await flushPromises();
    expect(document.body.textContent).toContain('No structured result');
    wrapper.unmount();
  });

  it('omits MCP rendering for non-MCP runs (regression)', async () => {
    readResource.mockResolvedValue({
      uri: 'schedule://logs/daily-summary',
      text: JSON.stringify([{ fileName: '20260429T140000Z.log', status: 'success', content: 'ok' }]),
    });

    const wrapper = mountDrawer();
    await flushPromises();
    await expandFirstRun();

    expect(document.body.textContent).not.toContain('MCP action:');
    wrapper.unmount();
  });

  it('Clear history button calls Schedule clear_history with the schedule id', async () => {
    readResource.mockResolvedValue({
      uri: 'schedule://logs/daily-summary',
      text: JSON.stringify([{ fileName: '20260429T140000Z.log', content: 'ok' }]),
    });

    const wrapper = mountDrawer();
    await flushPromises();
    const clearButton = Array.from(document.body.querySelectorAll('button'))
      .find((button) => button.textContent?.includes('Clear history'));
    clearButton?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    await flushPromises();
    const clearButtons = Array.from(document.body.querySelectorAll('button'))
      .filter((button) => button.textContent?.includes('Clear history'))
    const confirmButton = clearButtons[clearButtons.length - 1];
    const callsBefore = readResource.mock.calls.length;
    confirmButton?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    await flushPromises();

    expect(callTool).toHaveBeenCalledWith('Schedule', { action: 'clear_history', id: 'daily-summary' });
    expect(readResource.mock.calls.length).toBeGreaterThan(callsBefore);
    expect(readResource.mock.calls[readResource.mock.calls.length - 1]?.[0]).toBe('schedule://logs/daily-summary');
    wrapper.unmount();
  });

  it('Clear history button is hidden when runs list is empty', async () => {
    readResource.mockResolvedValue({ uri: 'schedule://logs/daily-summary', text: '[]' });

    const wrapper = mountDrawer();
    await flushPromises();

    expect(document.body.textContent).not.toContain('Clear history');
    wrapper.unmount();
  });
});
