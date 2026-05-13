import type { ComputedRef } from 'vue';
import { computed, defineComponent, getCurrentScope, h, onScopeDispose, ref, render } from 'vue';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';

export interface UnsavedChangesGuard {
  isDirty: ComputedRef<boolean>;
  markDirty: () => void;
  markClean: () => void;
  /** Returns true if the caller may proceed with the destructive action; false if user cancelled. */
  confirmIfDirty: () => Promise<boolean>;
  /** Optional convenience for guarding window beforeunload. Wires up a beforeunload listener while active; returns a cleanup function. */
  attachBeforeUnload: () => () => void;
}

type UnsavedChangesGuardOptions = {
  /** Custom dialog title. Defaults to "You have unsaved changes". */
  title?: string;
  /** Custom dialog body text. Defaults to "Your changes will be lost. Discard?" */
  body?: string;
  /** Discard button label. Defaults to "Discard". */
  discardLabel?: string;
  /** Cancel button label. Defaults to "Cancel". */
  cancelLabel?: string;
};

const UnsavedChangesGuardDialog = defineComponent({
  name: 'UnsavedChangesGuardDialog',
  props: {
    open: { type: Boolean, required: true },
    title: { type: String, required: true },
    body: { type: String, required: true },
    discardLabel: { type: String, required: true },
    cancelLabel: { type: String, required: true },
  },
  emits: {
    cancel: () => true,
    discard: () => true,
    'update:open': (_value: boolean) => true,
  },
  setup(props, { emit }) {
    function close(value: boolean) {
      emit('update:open', value);
      if (!value) {
        emit('cancel');
      }
    }

    return () => h(Dialog, { open: props.open, 'onUpdate:open': close }, () => (
      props.open
        ? h(DialogContent, { class: 'sm:max-w-md' }, () => [
          h(DialogHeader, () => [
            h(DialogTitle, () => props.title),
            h(DialogDescription, () => props.body),
          ]),
          h(DialogFooter, () => [
            h(Button, { type: 'button', variant: 'ghost', onClick: () => emit('cancel') }, () => props.cancelLabel),
            h(Button, { type: 'button', variant: 'destructive', onClick: () => emit('discard') }, () => props.discardLabel),
          ]),
        ])
        : null
    ));
  },
});

export function useUnsavedChangesGuard(options: UnsavedChangesGuardOptions = {}): UnsavedChangesGuard {
  const dirty = ref(false);
  const dialogOpen = ref(false);
  const title = options.title ?? 'You have unsaved changes';
  const body = options.body ?? 'Your changes will be lost. Discard?';
  const discardLabel = options.discardLabel ?? 'Discard';
  const cancelLabel = options.cancelLabel ?? 'Cancel';
  let container: HTMLDivElement | null = null;
  let pendingResolve: ((value: boolean) => void) | null = null;

  function markDirty() {
    dirty.value = true;
  }

  function markClean() {
    dirty.value = false;
  }

  function resolveDialog(value: boolean) {
    if (!pendingResolve) {
      return;
    }
    const resolve = pendingResolve;
    pendingResolve = null;
    dialogOpen.value = false;
    if (value) {
      markClean();
    }
    resolve(value);
  }

  function cleanupDialog() {
    if (container) {
      render(null, container);
      container.remove();
      container = null;
    }
    dialogOpen.value = false;
  }

  function ensureDialog() {
    if (container || typeof document === 'undefined') {
      return;
    }
    container = document.createElement('div');
    container.setAttribute('data-unsaved-guard-host', 'true');
    document.body.appendChild(container);
    const Host = defineComponent({
      name: 'UnsavedChangesGuardHost',
      setup() {
        return () => h(UnsavedChangesGuardDialog, {
          open: dialogOpen.value,
          title,
          body,
          discardLabel,
          cancelLabel,
          onCancel: () => resolveDialog(false),
          onDiscard: () => resolveDialog(true),
          'onUpdate:open': (value: boolean) => {
            dialogOpen.value = value;
          },
        });
      },
    });
    render(h(Host), container);
  }

  /**
   * Reject reentrant confirmations instead of queuing them. Each surface owns its own
   * guard instance, so overlapping prompts from one instance indicate a caller bug.
   */
  function confirmIfDirty() {
    if (pendingResolve) {
      return Promise.reject(new Error('useUnsavedChangesGuard: confirmIfDirty already pending'));
    }
    if (!dirty.value) {
      return Promise.resolve(true);
    }
    ensureDialog();
    dialogOpen.value = true;
    return new Promise<boolean>((resolve) => {
      pendingResolve = resolve;
    });
  }

  function attachBeforeUnload() {
    const listener = (event: BeforeUnloadEvent) => {
      if (!dirty.value) {
        return;
      }
      event.preventDefault();
      event.returnValue = '';
    };
    window.addEventListener('beforeunload', listener);
    return () => window.removeEventListener('beforeunload', listener);
  }

  if (getCurrentScope()) {
    onScopeDispose(cleanupDialog);
  }

  return {
    isDirty: computed(() => dirty.value),
    markDirty,
    markClean,
    confirmIfDirty,
    attachBeforeUnload,
  };
}
