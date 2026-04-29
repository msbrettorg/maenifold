// @vitest-environment jsdom
import { mount } from '@vue/test-utils';
import { nextTick } from 'vue';
import { describe, expect, it, vi } from 'vitest';
import PermissionDecisionDialog from '../PermissionDecisionDialog.vue';
import { DialogContent } from '../ui/dialog';
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
  const selectedScopeClass = 'border-brand';

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

  it('visibly highlights the default selected scope', async () => {
    const wrapper = mount(PermissionDecisionDialog, {
      props: { request: request(), totalPending: 1 },
    });

    await wrapper.get('.option-allow_always').trigger('click');

    expect(wrapper.get('[data-scope="agent"]').classes()).toContain(selectedScopeClass);
    expect(wrapper.get('[data-scope="session"]').classes()).not.toContain(selectedScopeClass);
    expect(wrapper.get('[data-scope="forever"]').classes()).not.toContain(selectedScopeClass);
  });

  it('moves the visible highlight when a different scope is selected', async () => {
    const wrapper = mount(PermissionDecisionDialog, {
      props: { request: request(), totalPending: 1 },
    });

    await wrapper.get('.option-allow_always').trigger('click');
    await wrapper.get('[data-scope="forever"]').trigger('click');

    expect(wrapper.get('[data-scope="forever"]').classes()).toContain(selectedScopeClass);
    expect(wrapper.get('[data-scope="agent"]').classes()).not.toContain(selectedScopeClass);
  });
});

describe('PermissionDecisionDialog modal dismissal', () => {
  function cancelableDismissEvent() {
    return { preventDefault: vi.fn() };
  }

  it('keeps the permission dialog open when pointer-down-outside fires', async () => {
    const wrapper = mount(PermissionDecisionDialog, {
      props: { request: request(), totalPending: 1 },
    });
    const event = cancelableDismissEvent();

    wrapper.getComponent(DialogContent).vm.$emit('pointer-down-outside', event);
    await nextTick();

    expect(event.preventDefault).toHaveBeenCalledOnce();
    expect(wrapper.find('.permission-dialog').exists()).toBe(true);
    expect(wrapper.text()).toContain('Read file');
    expect(wrapper.emitted('select')).toBeUndefined();
    expect(wrapper.emitted('cancel')).toBeUndefined();
  });

  it('keeps the permission dialog open when escape-key-down fires', async () => {
    const wrapper = mount(PermissionDecisionDialog, {
      props: { request: request(), totalPending: 1 },
    });
    const event = cancelableDismissEvent();

    wrapper.getComponent(DialogContent).vm.$emit('escape-key-down', event);
    await nextTick();

    expect(event.preventDefault).toHaveBeenCalledOnce();
    expect(wrapper.find('.permission-dialog').exists()).toBe(true);
    expect(wrapper.text()).toContain('Read file');
    expect(wrapper.emitted('select')).toBeUndefined();
    expect(wrapper.emitted('cancel')).toBeUndefined();
  });

  it('still emits the selected permission option when an action button is clicked', async () => {
    const wrapper = mount(PermissionDecisionDialog, {
      props: { request: request(), totalPending: 1 },
    });

    await wrapper.get('.option-allow_once').trigger('click');

    expect(wrapper.emitted('select')).toEqual([['allow-once']]);
  });
});
