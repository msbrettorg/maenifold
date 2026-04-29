// @vitest-environment jsdom
import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ref } from 'vue';
import GateStageActions from '../GateStageActions.vue';
import type { GateTask } from '../../lib/gate';

const mockCallTool = vi.fn();

vi.mock('../../composables/usePipelineMcp', () => ({
  getSharedMcpClient: vi.fn(() => ({
    connected: ref(true),
    connect: vi.fn(),
    error: ref(null),
    onNotification: vi.fn(() => vi.fn()),
    onDisconnect: vi.fn(() => vi.fn()),
    callTool: mockCallTool,
  })),
}));

const dialogStubs = {
  Dialog: { template: '<div><slot /></div>', props: ['open'] },
  DialogContent: { template: '<div><slot /></div>' },
  DialogHeader: { template: '<div><slot /></div>' },
  DialogTitle: { template: '<h2><slot /></h2>' },
  DialogDescription: { template: '<p><slot /></p>' },
  DialogFooter: { template: '<div><slot /></div>' },
};

const task: GateTask = {
  taskId: 'T-244.11',
  canonicalTaskId: 'T-244.11',
  title: 'Gate actions',
  pipeline: 'sdd',
  state: 'swe',
  stages: ['pending', 'swe', 'red-team', 'blue-team', 'done'],
  attempts: 1,
  location: 'todo',
  path: '/repo/todo/T-244.11-gate-actions.md',
  issue: '',
};

function mockGateSuccess() {
  mockCallTool.mockImplementation((name: string) => {
    if (name === 'GatewayIndex') {
      return Promise.resolve({
        commands: [
          { name: 'gate_Create' },
          { name: 'gate_Status' },
          { name: 'gate_Sync' },
          { name: 'gate_History' },
          { name: 'gate_TaskBody' },
          { name: 'gate_Append' },
          { name: 'gate_Pass' },
          { name: 'gate_Fail' },
        ],
      });
    }
    return Promise.resolve({ ok: true });
  });
}

function mountActions() {
  return mount(GateStageActions, {
    props: { task, root: '/repo' },
    global: { stubs: dialogStubs },
  });
}

describe('GateStageActions', () => {
  beforeEach(() => {
    mockCallTool.mockReset();
    mockGateSuccess();
  });

  it('requires append notes and calls gate_Append on submit', async () => {
    const wrapper = mountActions();

    await wrapper.find('[data-testid="gate-append-trigger"]').trigger('click');
    await wrapper.find('[data-testid="gate-append-form"]').trigger('submit');
    expect(wrapper.text()).toContain('Notes are required.');
    expect(mockCallTool).not.toHaveBeenCalledWith('GatewayExecute', expect.objectContaining({
      command: 'gate_Append',
    }));

    await wrapper.find('[data-testid="gate-append-notes"]').setValue('Implemented the requested change.');
    await wrapper.find('[data-testid="gate-append-form"]').trigger('submit');
    await flushPromises();

    expect(mockCallTool).toHaveBeenCalledWith('GatewayExecute', {
      domain: 'maenifold.gate',
      command: 'gate_Append',
      parameters: {
        root: '/repo',
        taskId: 'T-244.11',
        agent: 'swe',
        notes: 'Implemented the requested change.',
      },
    });
    expect(wrapper.emitted('completed')).toBeTruthy();
  });

  it('requires pass notes and calls gate_Pass with the active stage agent', async () => {
    const wrapper = mountActions();

    await wrapper.find('[data-testid="gate-pass-trigger"]').trigger('click');
    expect(wrapper.text()).toContain('advance from `swe` to `red-team`');
    expect(wrapper.find('[data-testid="gate-pass-confirm"]').attributes('autofocus')).toBeDefined();

    await wrapper.find('[data-testid="gate-pass-form"]').trigger('submit');
    expect(wrapper.text()).toContain('Notes are required.');

    await wrapper.find('[data-testid="gate-pass-notes"]').setValue('PASS: ready for review.');
    await wrapper.find('[data-testid="gate-pass-form"]').trigger('submit');
    await flushPromises();

    expect(mockCallTool).toHaveBeenCalledWith('GatewayExecute', expect.objectContaining({
      command: 'gate_Pass',
      parameters: expect.objectContaining({
        root: '/repo',
        taskId: 'T-244.11',
        agent: 'swe',
        notes: 'PASS: ready for review.',
      }),
    }));
  });

  it('requires fail notes, explains the fail edge, and defaults focus to Cancel', async () => {
    const wrapper = mountActions();

    await wrapper.find('[data-testid="gate-fail-trigger"]').trigger('click');
    expect(wrapper.text()).toContain('Failing `swe` routes this task to `pending`');
    expect(wrapper.find('[data-testid="gate-fail-cancel"]').attributes('autofocus')).toBeDefined();

    await wrapper.find('[data-testid="gate-fail-form"]').trigger('submit');
    expect(wrapper.text()).toContain('Notes are required.');

    await wrapper.find('[data-testid="gate-fail-notes"]').setValue('FAIL: missing coverage.');
    await wrapper.find('[data-testid="gate-fail-form"]').trigger('submit');
    await flushPromises();

    expect(mockCallTool).toHaveBeenCalledWith('GatewayExecute', expect.objectContaining({
      command: 'gate_Fail',
      parameters: expect.objectContaining({
        root: '/repo',
        taskId: 'T-244.11',
        agent: 'swe',
        notes: 'FAIL: missing coverage.',
      }),
    }));
  });
});
