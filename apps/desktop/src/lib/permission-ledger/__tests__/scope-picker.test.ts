// @vitest-environment jsdom
import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import PermissionDecisionDialog from '../../../components/PermissionDecisionDialog.vue';
import type { PermissionRequest } from '../../types';

function request(kind = 'read', rawInput: Record<string, unknown> = { path: '/repo/file.txt' }): PermissionRequest {
  return {
    sessionId: 'session-1',
    toolCall: {
      toolCallId: 'tool-1',
      title: 'Read file',
      kind,
      status: 'pending',
      rawInput,
    },
    options: [
      { optionId: 'allow-once', kind: 'allow_once', name: 'Allow once' },
      { optionId: 'allow-always', kind: 'allow_always', name: 'Allow always' },
    ],
  };
}

describe('PermissionDecisionDialog', () => {
  it('reveals a scope picker instead of forwarding an _always option immediately', async () => {
    const wrapper = mount(PermissionDecisionDialog, {
      props: { request: request(), totalPending: 1 },
    });

    await wrapper.get('.option-allow_always').trigger('click');

    expect(wrapper.find('[data-test="permission-scope-picker"]').exists()).toBe(true);
    expect(wrapper.emitted('select')).toBeUndefined();
  });

  it('defaults execute and delete requests to session scope', async () => {
    const wrapper = mount(PermissionDecisionDialog, {
      props: { request: request('execute', { command: 'npm test', cwd: '/repo' }), totalPending: 1 },
    });

    await wrapper.get('.option-allow_always').trigger('click');

    expect(wrapper.get('[data-scope="session"]').attributes('aria-pressed')).toBe('true');
  });

  it('defaults read and fetch requests to agent scope', async () => {
    const wrapper = mount(PermissionDecisionDialog, {
      props: { request: request('fetch', { url: 'https://example.com/a' }), totalPending: 1 },
    });

    await wrapper.get('.option-allow_always').trigger('click');

    expect(wrapper.get('[data-scope="agent"]').attributes('aria-pressed')).toBe('true');
  });

  it('defaults memory and maenifold-root reads to forever scope', async () => {
    const wrapper = mount(PermissionDecisionDialog, {
      props: { request: request('read', { path: 'memory://notes/example' }), totalPending: 1 },
    });

    await wrapper.get('.option-allow_always').trigger('click');

    expect(wrapper.get('[data-scope="forever"]').attributes('aria-pressed')).toBe('true');
  });

  it('forwards the selected option with the chosen scope after confirmation', async () => {
    const wrapper = mount(PermissionDecisionDialog, {
      props: { request: request(), totalPending: 1 },
    });

    await wrapper.get('.option-allow_always').trigger('click');
    await wrapper.get('[data-test="confirm-permission-scope"]').trigger('click');

    expect(wrapper.emitted('select')).toEqual([['allow-always', 'agent']]);
  });
});
