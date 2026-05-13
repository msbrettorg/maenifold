import { onBeforeUnmount, watch } from 'vue';
import type { ComputedRef } from 'vue';

export function useEscCancelWhileStreaming(isLoading: ComputedRef<boolean>, onCancel: () => void): void {
  let detachEscListener: (() => void) | null = null;

  function attachWindowEscCancel(): void {
    if (detachEscListener) return;
    const handler = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return;
      if (event.isComposing) return;
      if (!isLoading.value) return;
      event.preventDefault();
      onCancel();
    };
    window.addEventListener('keydown', handler);
    detachEscListener = () => {
      window.removeEventListener('keydown', handler);
      detachEscListener = null;
    };
  }

  function detachWindowEscCancel(): void { detachEscListener?.(); }

  watch(isLoading, (loading) => { if (loading) attachWindowEscCancel(); else detachWindowEscCancel(); }, { immediate: true });
  onBeforeUnmount(() => detachWindowEscCancel());
}
