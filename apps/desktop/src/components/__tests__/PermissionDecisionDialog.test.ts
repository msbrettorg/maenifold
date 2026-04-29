// @vitest-environment jsdom
import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import PermissionDecisionDialog from '../PermissionDecisionDialog.vue';
import type { PermissionRequest } from '../../lib/types';

function request(): PermissionRequest {
  return {
    sessionId: 'session-1',
    toolCall: {
      toolCallId: 'tool-1',
      title: 'Read file',
      kind: 'read',
      status: 'pending',
      rawInput: { path: '/repo/file.txt' },
    },
    options: [
      { optionId: 'allow-once', kind: 'allow_once', name: 'Allow once' },
      { optionId: 'allow-always', kind: 'allow_always', name: 'Allow always' },
    ],
  };
}

describe('PermissionDecisionDialog scope picker', () => {
  it('renders the always-allow scope picker above the permission dialog and emits the selected scope', async () => {
    const wrapper = mount(PermissionDecisionDialog, {
      props: { request: request(), totalPending: 1 },
    });

    await wrapper.get('.option-allow_always').trigger('click');

    const scopePicker = wrapper.get('[data-test="permission-scope-picker"]');
    expect(scopePicker.isVisible()).toBe(true);
    expect(scopePicker.classes()).toContain('z-[1010]');
    expect(wrapper.emitted('select')).toBeUndefined();

    await wrapper.get('[data-scope="agent"]').trigger('click');
    await wrapper.get('[data-test="confirm-permission-scope"]').trigger('click');

    expect(wrapper.emitted('select')).toEqual([['allow-always', 'agent']]);
  });
});
