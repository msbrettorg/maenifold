// @vitest-environment jsdom
import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import PermissionDialog from '../PermissionDialog.vue';
import type { PermissionRequest } from '../../lib/types';

/** Inline stubs so Dialog content renders without Teleport/Portal. */
const dialogStubs = {
  Dialog: { template: '<div><slot /></div>', props: ['open'] },
  DialogContent: { template: '<div v-bind="$attrs"><slot /></div>' },
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
});
