import { beforeEach, describe, expect, it, vi } from 'vitest';
import { mount, flushPromises } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import SchedulesTab from '@/components/SchedulesTab.vue';
import { useConfigStore } from '@/stores/config';

const callTool = vi.fn();

vi.mock('@/composables/useMcpClient', () => ({
  useMcpClient: () => ({
    callTool,
    connect: vi.fn(),
    connected: { value: true },
    onNotification: vi.fn(),
  }),
}));

function seedAgents(): void {
  const config = useConfigStore();
  config.config.agents = { swe: { command: 'maenifold' } };
}

function job(id = 'daily') {
  return {
    id,
    description: id,
    schedule: '0 9 * * *',
    schedule_human: 'every day at 9am',
    agent: 'swe',
    prompt: 'Run scheduled maintenance',
    enabled: true,
    auto_grant_permissions: false,
    allowed_tools: [],
  };
}

async function mountTab(paused = false, jobs = [job()]) {
  callTool.mockImplementation((_tool: string, payload: { action: string }) => {
    if (payload.action === 'list') return Promise.resolve(jobs);
    if (payload.action === 'verify_installed') return Promise.resolve([]);
    if (payload.action === 'pause_status') {
      return Promise.resolve(paused
        ? { paused: true, pausedAt: '2026-05-14T00:00:00Z', schedulesPaused: jobs.length }
        : { paused: false });
    }
    return Promise.resolve({ message: `${payload.action} done` });
  });
  const wrapper = mount(SchedulesTab, {
    global: {
      stubs: {
        ScheduleRunsDrawer: true,
        ScheduleFormDialog: true,
      },
    },
  });
  await flushPromises();
  return wrapper;
}

describe('SchedulesTab pause-all row', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    callTool.mockReset();
    seedAgents();
  });

  it('renders Switch labeled "Pause all schedules" above platform banner', async () => {
    const wrapper = await mountTab();
    const text = wrapper.text();

    expect(text).toContain('Pause all schedules');
    expect(text.indexOf('Pause all schedules')).toBeLessThan(text.indexOf('Supported backends'));
    expect(wrapper.find('[role="switch"]').exists()).toBe(true);
  });

  it('Switch is disabled when jobs.length === 0', async () => {
    const wrapper = await mountTab(false, []);

    expect(wrapper.find('[role="switch"]').attributes('disabled')).toBeDefined();
  });

  it('Switch reflects isPausedAll from store', async () => {
    const wrapper = await mountTab(true);

    expect(wrapper.find('[role="switch"]').attributes('aria-checked')).toBe('true');
  });

  it('status text changes when paused vs running vs empty', async () => {
    const paused = await mountTab(true);
    expect(paused.text()).toContain('schedules paused since');

    const running = await mountTab(false);
    expect(running.text()).toContain('All schedules running normally.');

    const empty = await mountTab(false, []);
    expect(empty.text()).toContain('No schedules to pause.');
  });
});
