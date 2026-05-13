// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { nextTick } from 'vue';
import { useUnsavedChangesGuard } from '../useUnsavedChangesGuard';

async function openDirtyDialog() {
  const guard = useUnsavedChangesGuard();
  guard.markDirty();
  const confirmation = guard.confirmIfDirty();
  await nextTick();
  return { confirmation, guard };
}

function buttonNamed(name: string) {
  const buttons = Array.from(document.body.querySelectorAll('button'));
  const match = buttons.find((button) => button.textContent?.trim() === name);
  if (!match) {
    throw new Error(`Button not found: ${name}`);
  }
  return match;
}

describe('useUnsavedChangesGuard', () => {
  afterEach(() => {
    document.body.replaceChildren();
  });

  it('isDirty starts false', () => {
    const guard = useUnsavedChangesGuard();
    expect(guard.isDirty.value).toBe(false);
  });

  it('markDirty and markClean toggle isDirty', () => {
    const guard = useUnsavedChangesGuard();
    guard.markDirty();
    expect(guard.isDirty.value).toBe(true);
    guard.markClean();
    expect(guard.isDirty.value).toBe(false);
  });

  it('confirmIfDirty resolves true immediately when clean', async () => {
    const guard = useUnsavedChangesGuard();
    await expect(guard.confirmIfDirty()).resolves.toBe(true);
    expect(document.body.querySelector('[role="dialog"]')).toBeNull();
  });

  it('confirmIfDirty opens the dialog when dirty', async () => {
    const { confirmation } = await openDirtyDialog();
    expect(document.body.querySelector('[role="dialog"]')).not.toBeNull();
    buttonNamed('Cancel').click();
    await expect(confirmation).resolves.toBe(false);
  });

  it('discard resolves true and clears dirty state', async () => {
    const { confirmation, guard } = await openDirtyDialog();
    buttonNamed('Discard').click();
    await expect(confirmation).resolves.toBe(true);
    expect(guard.isDirty.value).toBe(false);
  });

  it('cancel resolves false and leaves dirty state intact', async () => {
    const { confirmation, guard } = await openDirtyDialog();
    buttonNamed('Cancel').click();
    await expect(confirmation).resolves.toBe(false);
    expect(guard.isDirty.value).toBe(true);
  });

  it('attachBeforeUnload returns cleanup and prevents unload while dirty', () => {
    const guard = useUnsavedChangesGuard();
    const cleanup = guard.attachBeforeUnload();
    expect(cleanup).toEqual(expect.any(Function));

    guard.markDirty();
    const event = new Event('beforeunload', { cancelable: true }) as BeforeUnloadEvent;
    Object.defineProperty(event, 'returnValue', { value: undefined, writable: true });
    const preventDefault = vi.spyOn(event, 'preventDefault');
    window.dispatchEvent(event);
    expect(preventDefault).toHaveBeenCalledOnce();
    expect(event.defaultPrevented).toBe(true);
    expect(event.returnValue).toBe('');

    cleanup();
    const afterCleanup = new Event('beforeunload', { cancelable: true });
    const afterPreventDefault = vi.spyOn(afterCleanup, 'preventDefault');
    window.dispatchEvent(afterCleanup);
    expect(afterPreventDefault).not.toHaveBeenCalled();
  });
});
