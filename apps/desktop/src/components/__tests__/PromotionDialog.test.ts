// @vitest-environment jsdom
import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import PromotionDialog from '../PromotionDialog.vue';

const toast = vi.hoisted(() => ({
  error: vi.fn(),
}));

const guard = vi.hoisted(() => {
  const vue = require('vue') as typeof import('vue');
  return {
    ref: vue.ref,
    markDirty: vi.fn(),
    markClean: vi.fn(),
    confirmIfDirty: vi.fn(async () => true),
    attachBeforeUnload: vi.fn(() => vi.fn()),
  };
});

const promotion = vi.hoisted(() => {
  const vue = require('vue') as typeof import('vue');
  return {
    busy: vue.ref(false),
    error: vue.ref(null),
    searching: vue.ref(false),
    writing: vue.ref(false),
    writtenUri: vue.ref(null),
    resetResultState: vi.fn(),
    searchMemories: vi.fn(async () => ({ results: [] })),
    readMemory: vi.fn(),
    appendMemory: vi.fn(),
    updateMemory: vi.fn(),
    writeMemory: vi.fn(async () => undefined),
  };
});

vi.mock('../../composables/useMemoryPromotion', () => ({
  buildPromotionPreview: () => 'Preview [[topic]] 2026-01-01T00:00:00.000Z',
  hasWikiLinkConnection: (value: string) => value.includes('[['),
  isStaleMemoryEditError: () => false,
  useMemoryPromotion: vi.fn(() => promotion),
}));

vi.mock('../../composables/useUnsavedChangesGuard', () => ({
  useUnsavedChangesGuard: vi.fn(() => ({
    isDirty: guard.ref(false),
    markDirty: guard.markDirty,
    markClean: guard.markClean,
    confirmIfDirty: guard.confirmIfDirty,
    attachBeforeUnload: guard.attachBeforeUnload,
  })),
}));

vi.mock('vue-sonner', () => ({ toast }));

const stubs = {
  Dialog: { template: '<div><slot /></div>', props: ['open'] },
  DialogContent: { template: '<div><slot /></div>' },
  DialogDescription: { template: '<p><slot /></p>' },
  DialogHeader: { template: '<div><slot /></div>' },
  DialogTitle: { template: '<h2><slot /></h2>' },
  ScrollArea: { template: '<div><slot /></div>' },
  PromotionDialogPreview: {
    emits: ['selectAction', 'close'],
    template: '<button data-testid="select-create" type="button" @click="$emit(\'selectAction\', \'create\')">Create</button>',
  },
  PromotionDialogConfirm: {
    emits: ['submit'],
    template: '<button data-testid="promotion-submit" type="button" @click="$emit(\'submit\')">Submit</button>',
  },
};

describe('PromotionDialog unsaved changes guard', () => {
  beforeEach(() => {
    guard.markDirty.mockClear();
    guard.markClean.mockClear();
    guard.confirmIfDirty.mockClear();
    guard.attachBeforeUnload.mockClear();
    guard.confirmIfDirty.mockResolvedValue(true);
    promotion.resetResultState.mockClear();
    promotion.searchMemories.mockClear();
    promotion.writeMemory.mockClear();
    toast.error.mockClear();
    promotion.writtenUri.value = null;
    document.body.innerHTML = '';
  });

  it('uses the shared guard for promotion edits, save success, and close', async () => {
    const wrapper = mount(PromotionDialog, {
      props: {
        pipelineId: 'pipeline-1',
        node: {
          nodeId: 'node-1',
          output: 'Result [[topic]]',
          status: 'completed',
          type: 'tool',
          toolName: 'Example',
          sessionId: 'session-1',
          attempts: 1,
        },
      },
      global: { stubs },
    });
    await flushPromises();

    await wrapper.get('[data-testid="select-create"]').trigger('click');
    await wrapper.get('input[placeholder="Note title"]').setValue('Promoted note');
    await wrapper.get('textarea').setValue('[[topic]]');
    expect(guard.markDirty).toHaveBeenCalled();

    const continueButton = wrapper.findAll('button').find((button) => button.text() === 'Continue');
    expect(continueButton).toBeTruthy();
    await continueButton!.trigger('click');
    await flushPromises();
    await wrapper.get('[data-testid="promotion-submit"]').trigger('click');
    await flushPromises();
    expect(promotion.writeMemory).toHaveBeenCalled();
    expect(guard.markClean).toHaveBeenCalled();

    await wrapper.findAll('button').find((button) => button.text() === 'Close')!.trigger('click');
    expect(guard.confirmIfDirty).toHaveBeenCalled();
  });

  it('surfaces empty Connections validation with toast, focus, and danger inline styling', async () => {
    const wrapper = mount(PromotionDialog, {
      attachTo: document.body,
      props: {
        pipelineId: 'pipeline-1',
        node: {
          nodeId: 'node-1',
          output: 'Result without links',
          status: 'completed',
          type: 'tool',
          toolName: 'Example',
          sessionId: 'session-1',
          attempts: 1,
        },
      },
      global: { stubs },
    });
    await flushPromises();

    await wrapper.get('[data-testid="select-create"]').trigger('click');
    await wrapper.get('input[placeholder="Note title"]').setValue('Promoted note');
    const connectionsField = wrapper.get('textarea');

    const continueButton = wrapper.findAll('button').find((button) => button.text() === 'Continue');
    expect(continueButton).toBeTruthy();
    await continueButton!.trigger('click');
    await flushPromises();

    expect(toast.error).toHaveBeenCalledWith('Add at least one connection before saving', {
      closeButton: true,
      duration: Infinity,
    });
    expect(document.activeElement).toBe(connectionsField.element);
    const connectionError = wrapper
      .findAll('[role="alert"]')
      .find((alert) => alert.text().toLowerCase().includes('connection'));
    expect(connectionError).toBeTruthy();
    expect(connectionError!.classes()).toContain('text-danger');

    wrapper.unmount();
  });
});
