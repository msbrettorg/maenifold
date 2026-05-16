// @vitest-environment jsdom
import { mount } from '@vue/test-utils';
import { nextTick } from 'vue';
import { describe, expect, it, vi } from 'vitest';
import PermissionDialog from '../PermissionDialog.vue';
import type { PermissionRequest } from '../../lib/types';

/** Inline stubs so Dialog content renders without Teleport/Portal. */
const dialogStubs = {
  Dialog: { template: '<div><slot /></div>', props: ['open'] },
  DialogContent: {
    emits: ['escape-key-down'],
    template: '<div v-bind="$attrs" tabindex="-1" @keydown.escape="$emit(\'escape-key-down\', $event)"><slot /></div>',
  },
  DialogHeader: { template: '<div class="dialog-header"><slot /></div>' },
  DialogTitle: { template: '<span><slot /></span>' },
  DialogDescription: { template: '<span class="queue-hint"><slot /></span>' },
};

function createPermissionRequest(): PermissionRequest {
  return {
    sessionId: 'session-1',
    toolCall: {
      toolCallId: 'tool-1',
      title: 'Run bash command',
      kind: 'bash',
      status: 'pending',
      rawInput: {
        command:
          'node ./scripts/really-long-command.js --with "arguments that keep growing" --and "grow again"',
        cwd: '/repo/apps/desktop',
      },
      rawOutput: {
        stdout: 'x'.repeat(2000),
      },
      content: [
        {
          type: 'stdout',
          text: 'line 1\nline 2\nline 3',
        },
      ],
    },
    options: [
      { optionId: 'allow-once', kind: 'allow_once', name: 'Allow once' },
      { optionId: 'allow-always', kind: 'allow_always', name: 'Always allow' },
      { optionId: 'reject-once', kind: 'reject_once', name: 'Reject once' },
    ],
  };
}

describe('PermissionDialog', () => {
  it('preserves queue messaging and selection semantics', async () => {
    const wrapper = mount(PermissionDialog, {
      props: {
        request: createPermissionRequest(),
        totalPending: 3,
      },
      global: { stubs: dialogStubs },
    });

    expect(wrapper.text()).toContain('Permission 1 of 3 — Run bash command');
    expect(wrapper.text()).toContain('3 pending');
    expect(wrapper.find('.permission-dialog').exists()).toBe(true);
    expect(wrapper.find('.dialog-content').exists()).toBe(true);
    expect(wrapper.find('.entry-details').exists()).toBe(true);
    expect(wrapper.find('.dialog-actions').exists()).toBe(true);

    await wrapper.get('.option-allow_once').trigger('click');
    expect(wrapper.emitted('select')).toEqual([['allow-once']]);

    await wrapper.get('.cancel-btn').trigger('click');
    expect(wrapper.emitted('cancel')).toEqual([[]]);
  });

  it('shows the scope picker before resolving an always decision', async () => {
    const wrapper = mount(PermissionDialog, {
      props: {
        request: createPermissionRequest(),
        totalPending: 1,
      },
      global: { stubs: dialogStubs },
    });

    await wrapper.get('.option-allow_always').trigger('click');

    expect(wrapper.find('[data-test="permission-scope-picker"]').exists()).toBe(true);
    expect(wrapper.text()).toContain('This session');
    expect(wrapper.text()).toContain('This agent');
    expect(wrapper.text()).toContain('Forever');
    expect(wrapper.emitted('select')).toBeUndefined();
  });

  it('avoids queue messaging for a single short request', () => {
    const wrapper = mount(PermissionDialog, {
      props: {
        request: createPermissionRequest(),
        totalPending: 1,
      },
      global: { stubs: dialogStubs },
    });

    expect(wrapper.text()).not.toContain('Permission 1 of');
  });

  it('renders entry details for raw payloads and content blocks', () => {
    const wrapper = mount(PermissionDialog, {
      props: {
        request: createPermissionRequest(),
        totalPending: 2,
      },
      global: { stubs: dialogStubs },
    });

    expect(wrapper.find('.entry-details').text()).toContain('really-long-command.js');
    expect(wrapper.find('.entry-details').text()).toContain('line 1');
    expect(wrapper.find('.entry-details').text()).toContain('stdout');
  });

  it('keeps Escape as a modal no-op without resolving, dismissing, or closing', async () => {
    const wrapper = mount(PermissionDialog, {
      props: {
        request: createPermissionRequest(),
        totalPending: 1,
      },
      attachTo: document.body,
      global: { stubs: dialogStubs },
    });
    const dialog = wrapper.get('.permission-dialog');
    const event = new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true });
    const stopPropagation = vi.spyOn(event, 'stopPropagation');

    dialog.element.dispatchEvent(event);
    await nextTick();

    expect(event.defaultPrevented).toBe(true);
    expect(stopPropagation).toHaveBeenCalledOnce();
    expect(wrapper.find('.permission-dialog').exists()).toBe(true);
    expect(wrapper.text()).toContain('Run bash command');
    expect(wrapper.emitted('select')).toBeUndefined();
    expect(wrapper.emitted('cancel')).toBeUndefined();

    wrapper.unmount();
  });
});
