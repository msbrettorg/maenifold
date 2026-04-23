import { ref, onScopeDispose } from 'vue';

interface ResizeOptions {
  initial: number;
  min: number;
  max: number;
}

export function useResizeHandle(options: ResizeOptions) {
  const width = ref(options.initial);
  const isResizing = ref(false);
  let cleanupDrag: (() => void) | null = null;

  function startResize(e: MouseEvent, growDirection: 'right' | 'left' = 'right') {
    e.preventDefault();
    isResizing.value = true;
    const startX = e.clientX;
    const startWidth = width.value;

    function onMove(ev: MouseEvent) {
      const delta = growDirection === 'right'
        ? ev.clientX - startX
        : startX - ev.clientX;
      width.value = Math.min(options.max, Math.max(options.min, startWidth + delta));
    }

    function onUp() {
      isResizing.value = false;
      document.removeEventListener('mousemove', onMove);
      document.removeEventListener('mouseup', onUp);
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
      cleanupDrag = null;
    }

    cleanupDrag?.();
    document.addEventListener('mousemove', onMove);
    document.addEventListener('mouseup', onUp);
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
    cleanupDrag = onUp;
  }

  onScopeDispose(() => cleanupDrag?.());

  return { width, isResizing, startResize };
}
