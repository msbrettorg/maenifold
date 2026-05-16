import { useEventListener } from '@vueuse/core';
import { ref } from 'vue';

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
      cleanupDrag?.();
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
      cleanupDrag = null;
    }

    cleanupDrag?.();
    const cleanupMove = useEventListener(document, 'mousemove', onMove);
    const cleanupUp = useEventListener(document, 'mouseup', onUp);
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
    cleanupDrag = () => {
      cleanupMove();
      cleanupUp();
    };
  }

  return { width, isResizing, startResize };
}
