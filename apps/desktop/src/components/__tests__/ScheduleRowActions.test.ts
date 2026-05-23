import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import ScheduleRowActions from '@/components/ScheduleRowActions.vue';

const gatewayExecute = vi.fn();

vi.mock('@/composables/useMcpClient', () => ({
  useMcpClient: () => ({
    gatewayExecute,
  }),
}));

function mountActions() {
  return mount(ScheduleRowActions, {
    props: { scheduleId: 'daily-summary' },
    attachTo: document.body,
  });
}

describe('ScheduleRowActions', () => {
  beforeEach(() => {
    gatewayExecute.mockReset();
    gatewayExecute.mockResolvedValue({
      action: 'backfill',
      scheduleId: 'daily-summary',
      plannedCount: 1,
      executedCount: 1,
      skippedCount: 0,
      runIds: ['20260510T090000Z.log'],
    });
  });

  afterEach(() => {
    document.body.innerHTML = '';
  });

  it('renders Backfill button', () => {
    const wrapper = mountActions();

    expect(wrapper.text()).toContain('Backfill');
  });

  it('opens AlertDialog with date-range inputs on click', async () => {
    const wrapper = mountActions();

    await wrapper.get('button').trigger('click');
    await flushPromises();

    expect(document.body.textContent).toContain('Backfill schedule runs?');
    expect(document.body.textContent).toContain('Since');
    expect(document.body.textContent).toContain('Until (optional)');
  });

  it('calls Schedule backfill with id, since, and (optional) until on confirm', async () => {
    const wrapper = mountActions();

    await wrapper.get('button').trigger('click');
    await flushPromises();
    const inputs = document.body.querySelectorAll('input');
    expect(inputs.length).toBeGreaterThanOrEqual(2);
    inputs[0].value = '2026-05-10T00:00:00Z';
    inputs[0].dispatchEvent(new Event('input', { bubbles: true }));
    inputs[1].value = '2026-05-12T00:00:00Z';
    inputs[1].dispatchEvent(new Event('input', { bubbles: true }));
    await flushPromises();
    const confirm = Array.from(document.body.querySelectorAll('button')).find((button) =>
      button.textContent?.includes('Run backfill'),
    );
    expect(confirm).toBeTruthy();
    confirm?.click();
    await flushPromises();

    expect(gatewayExecute).toHaveBeenCalledWith('maenifold.system', 'Schedule', {
      action: 'backfill',
      id: 'daily-summary',
      since: '2026-05-10T00:00:00Z',
      until: '2026-05-12T00:00:00Z',
    });
  });
});
