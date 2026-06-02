/* @vitest-environment jsdom */

import { flushPromises, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { nextTick } from 'vue';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const { executeGatewayMock, readGatewayResourceMock, listToolsMock, callToolMock } = vi.hoisted(
  () => ({
    executeGatewayMock: vi.fn(),
    readGatewayResourceMock: vi.fn(),
    listToolsMock: vi.fn(),
    callToolMock: vi.fn(),
  })
);

vi.mock('../lib/mcp/gateway.ts', () => ({
  executeGateway: executeGatewayMock,
  readGatewayResource: readGatewayResourceMock,
}));

vi.mock('../lib/mcp/client', async (importActual) => {
  const actual = await importActual<typeof import('../lib/mcp/client')>();
  return {
    ...actual,
    mcpHttpClient: {
      ...actual.mcpHttpClient,
      listTools: listToolsMock,
      callTool: callToolMock,
    },
  };
});

import ConfigOptionPicker from './ConfigOptionPicker.vue';
import SchedulesView from './SchedulesView.vue';
import { useSchedulesStore } from '../stores/schedules';

const mountedWrappers: Array<ReturnType<typeof mount>> = [];

function installLocalStorageShim() {
  const storage = new Map<string, string>();
  const shim = {
    getItem: vi.fn((key: string) => storage.get(key) ?? null),
    setItem: vi.fn((key: string, value: string) => {
      storage.set(key, value);
    }),
    removeItem: vi.fn((key: string) => {
      storage.delete(key);
    }),
    clear: vi.fn(() => {
      storage.clear();
    }),
  };

  vi.stubGlobal('localStorage', shim);
  return shim;
}

function scheduleFixture() {
  return {
    id: 'sched-nightly-knowledge-index',
    description: 'Nightly knowledge index',
    schedule: '0 2 * * *',
    schedule_human: 'daily at 2:00 AM',
    agent: 'swe',
    prompt: 'Rebuild the knowledge index',
    workdir: '/repo',
    auto_grant_permissions: false,
    allowed_tools: ['ReadMemory'],
    config_options: {
      model: 'claude-opus-4.6',
    },
    timeout_seconds: 600,
    enabled: true,
    created_at: '2026-06-02T00:00:00Z',
    updated_at: '2026-06-02T00:00:00Z',
    last_run: null,
  };
}

function configureMocks() {
  const task = scheduleFixture();
  executeGatewayMock.mockImplementation(
    async (_domain: string, _tool: string, parameters: Record<string, unknown>) => {
      switch (parameters.action) {
        case 'list':
          return [task];
        case 'options':
          return { mcp_actions: [] };
        case 'update':
          return { action: 'update', task };
        default:
          throw new Error(`Unexpected executeGateway action: ${String(parameters.action)}`);
      }
    }
  );
  readGatewayResourceMock.mockResolvedValue([]);
  listToolsMock.mockResolvedValue([
    {
      name: 'ReadMemory',
      description: 'Read memory entries',
    },
  ]);
  callToolMock.mockResolvedValue({
    content: [
      {
        type: 'text',
        text: JSON.stringify({
          agents: {
            swe: {
              command: 'npx',
              args: ['@github/copilot', '--acp'],
            },
          },
        }),
      },
    ],
  });
}

async function settle() {
  await nextTick();
  await flushPromises();
  await nextTick();
}

function mountSchedulesView() {
  const pinia = createPinia();
  setActivePinia(pinia);

  const wrapper = mount(SchedulesView, {
    global: {
      plugins: [pinia],
    },
  });
  mountedWrappers.push(wrapper);
  return wrapper;
}

async function openEditModal(wrapper: ReturnType<typeof mount>) {
  await settle();
  await wrapper.get('[aria-label="Edit schedule"]').trigger('click');
  await settle();
}

beforeEach(() => {
  installLocalStorageShim().clear();
  executeGatewayMock.mockReset();
  readGatewayResourceMock.mockReset();
  listToolsMock.mockReset();
  callToolMock.mockReset();
  configureMocks();
});

afterEach(() => {
  while (mountedWrappers.length > 0) {
    mountedWrappers.pop()?.unmount();
  }
  const storage = globalThis.localStorage as { clear?: () => void } | undefined;
  storage?.clear?.();
  document.body.innerHTML = '';
  vi.unstubAllGlobals();
});

describe('SchedulesView', () => {
  it('renders config option pickers for existing schedule config options when editing', async () => {
    const wrapper = mountSchedulesView();

    await openEditModal(wrapper);

    const pickers = wrapper.findAllComponents(ConfigOptionPicker);
    expect(pickers.length).toBeGreaterThan(0);
    expect((pickers[0]?.props('option') as { currentValue?: unknown } | undefined)?.currentValue).toBe(
      'claude-opus-4.6'
    );
  });

  it('updates editorForm.config_options.model when the picker emits a change event', async () => {
    const wrapper = mountSchedulesView();

    await openEditModal(wrapper);

    const pickers = wrapper.findAllComponents(ConfigOptionPicker);
    expect(pickers.length).toBeGreaterThan(0);
    await pickers[0]?.vm.$emit('change', 'gpt-5.5');
    await settle();

    expect((wrapper.vm as any).editorForm.config_options.model).toBe('gpt-5.5');
  });

  it('includes config_options in the schedule update payload when saving edits', async () => {
    const wrapper = mountSchedulesView();

    await openEditModal(wrapper);
    await wrapper.get('[aria-label="Schedule editor"] .btn.btn-primary').trigger('click');
    await settle();

    const updateCall = executeGatewayMock.mock.calls.find(
      (call) => (call[2] as Record<string, unknown> | undefined)?.action === 'update'
    );
    expect(updateCall?.[2]).toMatchObject({
      config_options: {
        model: 'claude-opus-4.6',
      },
    });
  });

  it('does not include config_options when togglePaused performs a partial update', async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const schedulesStore = useSchedulesStore();

    await schedulesStore.loadTasks();
    await schedulesStore.togglePaused('sched-nightly-knowledge-index');

    const updateCall = executeGatewayMock.mock.calls.find(
      (call) => (call[2] as Record<string, unknown> | undefined)?.action === 'update'
    );
    expect(updateCall?.[2]).toMatchObject({
      id: 'sched-nightly-knowledge-index',
      enabled: false,
    });
    expect(updateCall?.[2]).not.toHaveProperty('config_options');
  });
});
