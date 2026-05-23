import { beforeEach, describe, expect, it, vi } from 'vitest';
import { mount, flushPromises } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import SchedulesTab from '@/components/SchedulesTab.vue';
import { useConfigStore } from '@/stores/config';

const gatewayExecute = vi.fn();

vi.mock('@/composables/useMcpClient', () => ({
  useMcpClient: () => ({
    gatewayExecute,
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
  gatewayExecute.mockImplementation((_domain: string, _command: string, payload: { action: string }) => {
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
    gatewayExecute.mockReset();
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

describe('SchedulesTab column sort', () => {
  const sortedJobs = [
    { ...job('delta'), agent: 'sre', schedule: '0 6 * * *', last_run: { status: 'success', timestamp: '2026-05-14T08:00:00Z' } },
    { ...job('alpha'), agent: 'core', schedule: '15 4 * * 1', last_run: { status: 'success', timestamp: '2026-05-12T08:00:00Z' } },
    { ...job('echo'), agent: 'workflow', schedule: '30 2 * * *', last_run: { status: 'failed', timestamp: '2026-05-15T08:00:00Z', error: 'failed' } },
    { ...job('bravo'), agent: 'desktop', schedule: '45 1 * * 2', last_run: { status: 'success', timestamp: '2026-05-11T08:00:00Z' } },
    { ...job('zeta-null-run'), agent: 'agentless', schedule: '0 0 * * *', last_run: null },
  ];

  beforeEach(() => {
    setActivePinia(createPinia());
    gatewayExecute.mockReset();
    seedAgents();
  });

  function header(wrapper: Awaited<ReturnType<typeof mountTab>>, label: string) {
    const th = wrapper.findAll('th').find((item) => item.text().includes(label));
    expect(th).toBeDefined();
    return th!;
  }
  function names(wrapper: Awaited<ReturnType<typeof mountTab>>) {
    return wrapper.findAll('tbody tr').map((row) => row.findAll('td')[0].text());
  }
  async function clickHeader(wrapper: Awaited<ReturnType<typeof mountTab>>, label: string) {
    await header(wrapper, label).find('button').trigger('click');
    await flushPromises();
    await wrapper.vm.$nextTick();
  }

  it('default has no aria-sort', async () => {
    const wrapper = await mountTab(false, sortedJobs);
    const active = wrapper.findAll('th').filter((th) => ['ascending', 'descending'].includes(th.attributes('aria-sort') ?? ''));
    expect(active).toHaveLength(0);
  });

  it('click Name header sorts ascending', async () => {
    const wrapper = await mountTab(false, sortedJobs);
    await clickHeader(wrapper, 'Name');
    expect(names(wrapper)).toEqual(['alpha', 'bravo', 'delta', 'echo', 'zeta-null-run']);
    expect(header(wrapper, 'Name').attributes('aria-sort')).toBe('ascending');
  });

  it('click again sorts descending', async () => {
    const wrapper = await mountTab(false, sortedJobs);
    await clickHeader(wrapper, 'Name');
    await clickHeader(wrapper, 'Name');
    expect(names(wrapper)).toEqual(['zeta-null-run', 'echo', 'delta', 'bravo', 'alpha']);
    expect(header(wrapper, 'Name').attributes('aria-sort')).toBe('descending');
  });

  it('third click resets sort', async () => {
    const wrapper = await mountTab(false, sortedJobs);
    await clickHeader(wrapper, 'Name');
    await clickHeader(wrapper, 'Name');
    await clickHeader(wrapper, 'Name');
    const active = wrapper.findAll('th').filter((th) => ['ascending', 'descending'].includes(th.attributes('aria-sort') ?? ''));
    expect(names(wrapper)).toEqual(['delta', 'alpha', 'echo', 'bravo', 'zeta-null-run']);
    expect(active).toHaveLength(0);
  });

  it('aria-sort indicator renders on active column only', async () => {
    const wrapper = await mountTab(false, sortedJobs);
    await clickHeader(wrapper, 'Name');
    await clickHeader(wrapper, 'Agent');
    expect(header(wrapper, 'Agent').attributes('aria-sort')).toBe('ascending');
    expect(header(wrapper, 'Name').attributes('aria-sort')).toBeUndefined();
  });

  it('null/undefined fields sort to end', async () => {
    const wrapper = await mountTab(false, sortedJobs);
    await clickHeader(wrapper, 'Last run');
    expect(names(wrapper).at(-1)).toBe('zeta-null-run');
  });
});
