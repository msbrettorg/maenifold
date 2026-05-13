import { beforeEach, describe, expect, it } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { useConfirmDialogStore } from '../confirm-dialog';

describe('useConfirmDialogStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it('sequential confirms each resolve normally', async () => {
    const store = useConfirmDialogStore();

    const first = store.confirm('A');
    store.handleConfirm();
    await expect(first).resolves.toBe(true);

    const second = store.confirm('B');
    store.handleCancel();
    await expect(second).resolves.toBe(false);
  });

  it('concurrent confirms both surface and resolve independently', async () => {
    const store = useConfirmDialogStore();
    const first = store.confirm('A');
    const second = store.confirm('B');

    expect(store.title).toBe('A');
    store.handleConfirm();
    await expect(first).resolves.toBe(true);

    expect(store.title).toBe('B');
    store.handleCancel();
    await expect(second).resolves.toBe(false);
  });

  it('second confirm does NOT silently resolve the first', async () => {
    const store = useConfirmDialogStore();
    let firstSettled = false;

    const first = store.confirm('A');
    first.then(() => {
      firstSettled = true;
    });

    store.confirm('B');
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(firstSettled).toBe(false);

    store.handleCancel();
    await expect(first).resolves.toBe(false);
  });

  it('open is true while queue non-empty, false when empty', async () => {
    const store = useConfirmDialogStore();
    const first = store.confirm('A');
    const second = store.confirm('B');

    expect(store.open).toBe(true);
    store.handleConfirm();
    await expect(first).resolves.toBe(true);
    expect(store.open).toBe(true);

    store.handleCancel();
    await expect(second).resolves.toBe(false);
    expect(store.open).toBe(false);
  });

  it('title and description follow front of queue', async () => {
    const store = useConfirmDialogStore();
    const first = store.confirm('A', 'aDesc');
    const second = store.confirm('B', 'bDesc');

    expect(store.title).toBe('A');
    expect(store.description).toBe('aDesc');

    store.handleConfirm();
    await expect(first).resolves.toBe(true);
    expect(store.title).toBe('B');
    expect(store.description).toBe('bDesc');

    store.handleCancel();
    await expect(second).resolves.toBe(false);
  });
});
