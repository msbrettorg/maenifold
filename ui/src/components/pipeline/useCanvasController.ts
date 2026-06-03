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
// The single currently-selected node id (null when zero or multiple nodes are
// selected). Drives the teleported node inspector pane (T-600.6). Set by the
// canvas's flow→selection handler in useCanvasSync.
const selectedNodeId = ref<string | null>(null);
// ALL currently-selected node ids (0, 1, or many). `selectedNodeId` is null for
// a multi-selection, so the explicit multi-selection is surfaced here for batch
// surgery (T-600.5 batch rewind). Set by the canvas's flow→selection handler in
// useCanvasSync alongside `selectedNodeId`.
const selectedNodeIds = ref<string[]>([]);
// The node whose SemanticZoom overlay is open (null = overlay closed). Shared
// here because the inspector (which opens it) and the overlay (which renders it)
// are teleported into sibling subtrees and cannot reach each other via props.
const zoomNodeId = ref<string | null>(null);

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

  function setSelectedNodeId(next: string | null): void {
    selectedNodeId.value = next;
  }

  function setSelectedNodeIds(next: string[]): void {
    selectedNodeIds.value = next;
  }

  function openZoom(nodeId: string): void {
    zoomNodeId.value = nodeId;
  }

  function closeZoom(): void {
    zoomNodeId.value = null;
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
    selectedNodeId,
    selectedNodeIds,
    zoomNodeId,
    registerActions,
    clearActions,
    setDirection,
    togglePalette,
    setSelectedNodeId,
    setSelectedNodeIds,
    openZoom,
    closeZoom,
    autoLayout,
    fitView,
    deleteSelected,
  };
}
