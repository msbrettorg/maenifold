// @vitest-environment jsdom
import { flushPromises, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import SettingsMcpServers from '../SettingsMcpServers.vue';
import { useConfirmDialogStore } from '../../stores/confirm-dialog';
import { useConfigStore } from '../../stores/config';
import type { AgentsConfig } from '../../lib/types';

const guard = vi.hoisted(() => ({
  markDirty: vi.fn(),
  markClean: vi.fn(),
  confirmIfDirty: vi.fn(async () => true),
}));

vi.mock('../../composables/useUnsavedChangesGuard', () => ({
  useUnsavedChangesGuard: () => guard,
}));

const stubs = {
  Dialog: { props: ['open'], emits: ['update:open'], template: '<div v-if="open"><slot /></div>' },
  DialogContent: { template: '<div><slot /></div>' },
  DialogDescription: { template: '<p><slot /></p>' },
  DialogFooter: { template: '<footer><slot /></footer>' },
  DialogHeader: { template: '<header><slot /></header>' },
  DialogTitle: { template: '<h3><slot /></h3>' },
  Select: {
    props: ['modelValue'],
    emits: ['update:modelValue'],
    template: '<select :value="modelValue" @change="$emit(\'update:modelValue\', $event.target.value)"><slot /></select>',
  },
  SelectContent: { template: '<slot />' },
  SelectItem: { props: ['value'], template: '<option :value="value"><slot /></option>' },
  SelectTrigger: { template: '<span v-bind="$attrs"><slot /></span>' },
  SelectValue: { template: '<span />' },
  Edit: true,
  Plus: true,
  Trash2: true,
};

function mountSubject() {
  return mount(SettingsMcpServers, { global: { stubs } });
}

function setupConfig(config: AgentsConfig) {
  setActivePinia(createPinia());
  const configStore = useConfigStore();
  configStore.config = config;
  const writeConfig = vi.fn(async (updates: Partial<AgentsConfig>) => {
    configStore.config = { ...configStore.config, ...updates };
  });
  Object.assign(configStore, { writeConfig });
  return { configStore, writeConfig, confirmDialogStore: useConfirmDialogStore() };
}

describe('SettingsMcpServers', () => {
  beforeEach(() => {
    guard.markDirty.mockClear();
    guard.markClean.mockClear();
    guard.confirmIfDirty.mockClear();
    guard.confirmIfDirty.mockResolvedValue(true);
  });

  it('shows the empty state when no servers are configured', () => {
    setupConfig({ agents: {}, mcpServers: [] });
    const wrapper = mountSubject();

    expect(wrapper.text()).toContain("No MCP servers configured yet. Click 'Add server' to add one.");
  });

  it('renders configured servers as cards', () => {
    setupConfig({
      agents: {},
      mcpServers: [
        { type: 'stdio', name: 'filesystem', command: 'npx', args: ['-y', 'server'], env: { TOKEN: 'secret' } } as any,
        { type: 'http', name: 'remote', url: 'https://example.com/mcp' },
      ],
    });
    const wrapper = mountSubject();

    expect(wrapper.text()).toContain('filesystem');
    expect(wrapper.text()).toContain('Stdio');
    expect(wrapper.text()).toContain('npx · -y server');
    expect(wrapper.text()).toContain('1 env vars');
    expect(wrapper.text()).toContain('remote');
    expect(wrapper.text()).toContain('https://example.com/mcp');
  });

  it('confirms and removes a server from configStore', async () => {
    const { confirmDialogStore, writeConfig, configStore } = setupConfig({
      agents: {},
      mcpServers: [{ type: 'http', name: 'remote', url: 'https://example.com/mcp' }],
    });
    vi.spyOn(confirmDialogStore, 'confirm').mockResolvedValue(true);
    const wrapper = mountSubject();

    await wrapper.findAll('button').find((button) => button.text().includes('Remove'))!.trigger('click');
    await flushPromises();

    expect(confirmDialogStore.confirm).toHaveBeenCalledWith('Remove MCP server?', 'Remove "remote" from maenifold.json?');
    expect(writeConfig).toHaveBeenCalledWith({ mcpServers: [] });
    expect(configStore.config.mcpServers).toEqual([]);
  });

  it('opens the editor from the Add server button', async () => {
    setupConfig({ agents: {}, mcpServers: [] });
    const wrapper = mountSubject();

    await wrapper.findAll('button').find((button) => button.text().includes('Add server'))!.trigger('click');

    expect(wrapper.text()).toContain('Add MCP server');
    expect(wrapper.text()).toContain('Name');
  });

  it('shows Stdio command, args, and env fields by default', async () => {
    setupConfig({ agents: {}, mcpServers: [] });
    const wrapper = mountSubject();

    await wrapper.findAll('button').find((button) => button.text().includes('Add server'))!.trigger('click');

    expect(wrapper.text()).toContain('Command');
    expect(wrapper.text()).toContain('Args');
    expect(wrapper.text()).toContain('Env vars');
  });

  it('shows URL only for Http transport and hides env editing', async () => {
    setupConfig({
      agents: {},
      mcpServers: [{ type: 'http', name: 'remote', url: 'https://example.com/mcp' }],
    });
    const wrapper = mountSubject();

    await wrapper.findAll('button').find((button) => button.text().includes('Edit'))!.trigger('click');

    expect(wrapper.find('#mcp-server-url').exists()).toBe(true);
    expect(wrapper.text()).not.toContain('Env vars');
  });

  it('submits a valid stdio form and adds the server', async () => {
    const { writeConfig } = setupConfig({ agents: {}, mcpServers: [] });
    const wrapper = mountSubject();

    await wrapper.findAll('button').find((button) => button.text().includes('Add server'))!.trigger('click');
    await wrapper.get('#mcp-server-name').setValue('filesystem');
    await wrapper.get('#mcp-server-command').setValue('npx');
    await wrapper.get('#mcp-server-args').setValue('-y server');
    await wrapper.findAll('button').find((button) => button.text().includes('Add env var'))!.trigger('click');
    await wrapper.get('[aria-label="Env key"]').setValue('TOKEN');
    await wrapper.get('[aria-label="Env value"]').setValue('secret');
    await wrapper.get('form').trigger('submit');
    await flushPromises();

    expect(writeConfig).toHaveBeenCalledWith({
      mcpServers: [{ type: 'stdio', name: 'filesystem', command: 'npx', args: ['-y', 'server'], env: { TOKEN: 'secret' } }],
    });
  });

  it('shows an inline error for duplicate names', async () => {
    const { writeConfig } = setupConfig({
      agents: {},
      mcpServers: [{ type: 'http', name: 'remote', url: 'https://example.com/mcp' }],
    });
    const wrapper = mountSubject();

    await wrapper.findAll('button').find((button) => button.text().includes('Add server'))!.trigger('click');
    await wrapper.get('#mcp-server-name').setValue('remote');
    await wrapper.get('#mcp-server-command').setValue('npx');
    await wrapper.get('form').trigger('submit');

    expect(wrapper.text()).toContain('Name must be unique.');
    expect(writeConfig).not.toHaveBeenCalled();
  });

  it('cancels without saving dirty edits', async () => {
    const { writeConfig } = setupConfig({ agents: {}, mcpServers: [] });
    const wrapper = mountSubject();

    await wrapper.findAll('button').find((button) => button.text().includes('Add server'))!.trigger('click');
    await wrapper.get('#mcp-server-name').setValue('draft');
    await wrapper.findAll('button').find((button) => button.text() === 'Cancel')!.trigger('click');
    await flushPromises();

    expect(guard.confirmIfDirty).toHaveBeenCalled();
    expect(wrapper.text()).not.toContain('Add MCP server');
    expect(writeConfig).not.toHaveBeenCalled();
  });

  it('pre-populates the edit form for an existing server', async () => {
    setupConfig({
      agents: {},
      mcpServers: [{ type: 'stdio', name: 'filesystem', command: 'npx', args: ['-y', 'server'] }],
    });
    const wrapper = mountSubject();

    await wrapper.findAll('button').find((button) => button.text().includes('Edit'))!.trigger('click');

    expect((wrapper.get('#mcp-server-name').element as HTMLInputElement).value).toBe('filesystem');
    expect((wrapper.get('#mcp-server-command').element as HTMLInputElement).value).toBe('npx');
    expect((wrapper.get('#mcp-server-args').element as HTMLTextAreaElement).value).toBe('-y server');
  });
});
