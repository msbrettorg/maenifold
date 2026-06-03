// Canvas controller — the decoupling seam between the teleported toolbar and the
// Vue Flow canvas.
//
// The toolbar is teleported into `#pipeline-toolbar` (a different subtree from
// the canvas) so it cannot reach the canvas through props/emits. This module is
// a module-level singleton of reactive state plus a slot for the canvas's
// imperative action handlers: the canvas registers its handlers on mount; the
// toolbar reads the shared state and invokes the handlers. Both call
// `useCanvasController()` and receive the same refs.

import { ref } from 'vue';
import { DEFAULT_DIRECTION, type Direction } from './canvasBridge';

export interface CanvasActions {
  autoLayout: () => void;
  fitView: () => void;
  deleteSelected: () => void;
}

const direction = ref<Direction>(DEFAULT_DIRECTION);
const paletteOpen = ref(false);
const hasSelection = ref(false);

let actions: CanvasActions | null = null;

export function useCanvasController() {
  function registerActions(next: CanvasActions): void {
    actions = next;
  }

  function clearActions(): void {
    actions = null;
  }

  function setDirection(next: Direction): void {
    direction.value = next;
  }

  function togglePalette(): void {
    paletteOpen.value = !paletteOpen.value;
  }

  function autoLayout(): void {
    actions?.autoLayout();
  }

  function fitView(): void {
    actions?.fitView();
  }

  function deleteSelected(): void {
    actions?.deleteSelected();
  }

  return {
    direction,
    paletteOpen,
    hasSelection,
    registerActions,
    clearActions,
    setDirection,
    togglePalette,
    autoLayout,
    fitView,
    deleteSelected,
  };
}
