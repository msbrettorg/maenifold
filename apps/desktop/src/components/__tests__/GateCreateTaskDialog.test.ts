// @vitest-environment jsdom
import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ref } from 'vue';
import GateCreateTaskDialog from '../GateCreateTaskDialog.vue';
import type { GateTask } from '../../lib/gate';

const mockCallTool = vi.fn();
const gateWriteTaskFile = vi.fn();

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

const selectStubs = {
  Select: {
    props: ['modelValue'],
    emits: ['update:modelValue'],
    template: '<div data-testid="select-stub"><slot /></div>',
  },
  SelectTrigger: { template: '<button type="button"><slot /></button>' },
  SelectValue: { props: ['placeholder'], template: '<span>{{ placeholder }}</span>' },
  SelectContent: { template: '<div><slot /></div>' },
  SelectItem: {
    props: ['value'],
    template: '<button type="button" :data-testid="`pipeline-option-${value}`" @click="$parent.$emit(\'update:modelValue\', value)"><slot /></button>',
  },
};

const tasks: GateTask[] = [
  {
    taskId: 'T-244.11',
    canonicalTaskId: 'T-244.11',
    title: 'Existing task',
    pipeline: 'tdd',
    state: 'swe',
    stages: ['swe'],
    attempts: 1,
    location: 'todo',
    path: '/repo/todo/T-244.11-existing-task.md',
    issue: '',
  },
];

function mockGateSuccess() {
  mockCallTool.mockImplementation((name: string) => {
    if (name === 'GatewayIndex') {
      return Promise.resolve({
        commands: [
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

function mountDialog(extraProps = {}) {
  return mount(GateCreateTaskDialog, {
    props: {
      open: true,
      root: '/repo',
      tasks,
      ...extraProps,
    },
    global: { stubs: { ...dialogStubs, ...selectStubs } },
  });
}

describe('GateCreateTaskDialog', () => {
  beforeEach(() => {
    mockCallTool.mockReset();
    gateWriteTaskFile.mockReset();
    mockGateSuccess();
    Object.defineProperty(window, 'electronAPI', {
      value: { gateWriteTaskFile },
      configurable: true,
    });
    gateWriteTaskFile.mockResolvedValue({ path: '/repo/todo/T-244.12-create-task-form.md' });
  });

  it('prefills the next sprint id and lists supported pipelines', () => {
    const wrapper = mountDialog();

    expect((wrapper.find('[data-testid="gate-create-id"]').element as HTMLInputElement).value).toBe('T-244.12');
    expect(wrapper.text()).toContain('sdd');
    expect(wrapper.text()).toContain('tdd');
    expect(wrapper.text()).toContain('gsd');
    expect(wrapper.text()).toContain('lightweight');
  });

  it('validates id format and collisions inline', async () => {
    const wrapper = mountDialog();

    await wrapper.find('[data-testid="gate-create-id"]').setValue('244.12');
    await wrapper.find('[data-testid="gate-create-title"]').setValue('New task');
    await wrapper.find('[data-testid="gate-create-form"]').trigger('submit');
    expect(wrapper.text()).toContain('Task id must match T-244.{n}.');

    await wrapper.find('[data-testid="gate-create-id"]').setValue('T-244.11');
    await wrapper.find('[data-testid="gate-create-form"]').trigger('submit');
    expect(wrapper.text()).toContain('Task id T-244.11 already exists.');
  });

  it('writes task markdown and syncs the gate on submit', async () => {
    const wrapper = mountDialog();

    await wrapper.find('[data-testid="gate-create-title"]').setValue('Create task form');
    await wrapper.find('[data-testid="gate-create-body"]').setValue('Task body markdown.');
    await wrapper.find('[data-testid="gate-create-form"]').trigger('submit');
    await flushPromises();

    expect(gateWriteTaskFile).toHaveBeenCalledWith({
      root: '/repo',
      filename: 'T-244.12-create-task-form.md',
      content: expect.stringContaining('id: T-244.12'),
    });
    expect(gateWriteTaskFile.mock.calls[0][0].filename).toMatch(/^T-\d+\.\d+-[a-z0-9-]+\.md$/);
    expect(gateWriteTaskFile.mock.calls[0][0].content).toContain('title: Create task form');
    expect(gateWriteTaskFile.mock.calls[0][0].content).toContain('pipeline: tdd');
    expect(gateWriteTaskFile.mock.calls[0][0].content).toContain('priority: P0');
    expect(gateWriteTaskFile.mock.calls[0][0].content).toContain('sprint: 244');
    expect(gateWriteTaskFile.mock.calls[0][0].content).toContain('Task body markdown.');
    expect(mockCallTool).toHaveBeenCalledWith('GatewayExecute', {
      domain: 'maenifold.gate',
      command: 'gate_Sync',
      parameters: { root: '/repo', rebuild: true },
    });
    expect(wrapper.emitted('created')?.[0]).toEqual(['T-244.12']);
  });

  it('surfaces filename regex validation failures from the gate task IPC', async () => {
    gateWriteTaskFile.mockRejectedValueOnce(new Error('invalid filename'));
    const wrapper = mountDialog();

    await wrapper.find('[data-testid="gate-create-title"]').setValue('Create task form');
    await wrapper.find('[data-testid="gate-create-form"]').trigger('submit');
    await flushPromises();

    expect(wrapper.text()).toContain('invalid filename');
    expect(mockCallTool).not.toHaveBeenCalledWith('GatewayExecute', expect.anything());
  });

  it('surfaces no-clobber task file errors from the gate task IPC', async () => {
    gateWriteTaskFile.mockRejectedValueOnce(new Error('task file already exists'));
    const wrapper = mountDialog();

    await wrapper.find('[data-testid="gate-create-title"]').setValue('Create task form');
    await wrapper.find('[data-testid="gate-create-form"]').trigger('submit');
    await flushPromises();

    expect(wrapper.text()).toContain('task file already exists');
    expect(mockCallTool).not.toHaveBeenCalledWith('GatewayExecute', expect.anything());
  });
});
