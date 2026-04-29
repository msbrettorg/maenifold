// @vitest-environment jsdom
import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import PermissionDialog from '../PermissionDialog.vue';
import type { PermissionRequest } from '../../lib/types';

const dialogStubs = {
  Dialog: { template: '<div><slot /></div>', props: ['open'] },
  DialogContent: { template: '<div v-bind="$attrs"><slot /></div>' },
  DialogHeader: { template: '<div class="dialog-header"><slot /></div>' },
  DialogTitle: { template: '<span><slot /></span>' },
  DialogDescription: { template: '<span class="queue-hint"><slot /></span>' },
};

function createFsWriteRequest(content = 'short content'): PermissionRequest {
  return {
    sessionId: 'session-1',
    toolCall: {
      toolCallId: 'tool-write-1',
      title: 'Write text file',
      kind: 'fs/write_text_file',
      status: 'pending',
      rawInput: {
        path: '/tmp/notes.md',
        content,
      },
    },
    options: [
      { optionId: 'allow-once', kind: 'allow_once', name: 'Allow once' },
      { optionId: 'allow-path', kind: 'allow_always', name: 'Always allow this path' },
      { optionId: 'deny-once', kind: 'reject_once', name: 'Deny' },
      { optionId: 'deny-always', kind: 'reject_always', name: 'Always deny' },
    ],
  };
}

describe('PermissionDialog fs/write presentation', () => {
  it('renders the target path for fs/write requests', () => {
    const wrapper = mount(PermissionDialog, {
      props: {
        request: createFsWriteRequest(),
        totalPending: 1,
      },
      global: { stubs: dialogStubs },
    });

    expect(wrapper.get('[data-testid="fs-write-target-path"]').text()).toBe('/tmp/notes.md');
  });

  it('shows a 200 character content preview with truncation', () => {
    const content = 'a'.repeat(220) + 'tail';
    const wrapper = mount(PermissionDialog, {
      props: {
        request: createFsWriteRequest(content),
        totalPending: 1,
      },
      global: { stubs: dialogStubs },
    });

    const preview = wrapper.get('[data-testid="fs-write-content-preview"]').text();
    expect(preview).toContain('a'.repeat(200));
    expect(preview).toContain('…');
    expect(preview).not.toContain('tail');
  });

  it('defaults focus to the deny button', () => {
    const wrapper = mount(PermissionDialog, {
      props: {
        request: createFsWriteRequest(),
        totalPending: 1,
      },
      global: { stubs: dialogStubs },
    });

    expect(wrapper.get('[data-testid="permission-deny-button"]').attributes('autofocus')).toBeDefined();
  });

  it('shows an always-allow-this-path action', () => {
    const wrapper = mount(PermissionDialog, {
      props: {
        request: createFsWriteRequest(),
        totalPending: 1,
      },
      global: { stubs: dialogStubs },
    });

    expect(wrapper.get('[data-testid="permission-allow-path-button"]').text()).toContain(
      'Always allow this path'
    );
  });

  it('shows an always-deny action', () => {
    const wrapper = mount(PermissionDialog, {
      props: {
        request: createFsWriteRequest(),
        totalPending: 1,
      },
      global: { stubs: dialogStubs },
    });

    expect(wrapper.get('[data-testid="permission-deny-always-button"]').text()).toContain(
      'Always deny'
    );
  });
});
