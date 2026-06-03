// Design-mode UI controller — the module-level singleton that decouples the Vue
// Flow canvas (which captures native context-menu / connect events) from the
// teleported design surfaces (context menu, node form, edge form), which live in
// a different DOM subtree and so cannot communicate via props/emits.
//
// Analogous to `useCanvasController`, but scoped to design-mode interaction
// state: which context menu is open and where, which node/edge form is open, and
// the flow-space cursor position used to seed a newly added node.

import { ref } from 'vue';
import type { XY } from '../canvasBridge';

export type ContextMenuKind = 'pane' | 'node' | 'edge';

export interface ContextMenuState {
  kind: ContextMenuKind;
  // Screen (client) coordinates for fixed-positioning the floating menu.
  x: number;
  y: number;
  // For node menus: the node id. For edge menus: the Vue Flow edge id (resolved
  // to a definition-array index by the consumer via `indexForFlowEdgeId`).
  targetId?: string;
  // Flow-space cursor position captured for a pane menu (seed for a new node).
  flowPos?: XY;
}

const menu = ref<ContextMenuState | null>(null);
const editingNodeId = ref<string | null>(null);
const editingEdgeIndex = ref<number | null>(null);

export function useDesignController() {
  function openPaneMenu(x: number, y: number, flowPos: XY): void {
    menu.value = { kind: 'pane', x, y, flowPos };
  }

  function openNodeMenu(x: number, y: number, nodeId: string): void {
    menu.value = { kind: 'node', x, y, targetId: nodeId };
  }

  function openEdgeMenu(x: number, y: number, flowEdgeId: string): void {
    menu.value = { kind: 'edge', x, y, targetId: flowEdgeId };
  }

  function closeMenu(): void {
    menu.value = null;
  }

  function openNodeForm(nodeId: string): void {
    menu.value = null;
    editingEdgeIndex.value = null;
    editingNodeId.value = nodeId;
  }

  function closeNodeForm(): void {
    editingNodeId.value = null;
  }

  function openEdgeForm(index: number): void {
    menu.value = null;
    editingNodeId.value = null;
    editingEdgeIndex.value = index;
  }

  function closeEdgeForm(): void {
    editingEdgeIndex.value = null;
  }

  function reset(): void {
    menu.value = null;
    editingNodeId.value = null;
    editingEdgeIndex.value = null;
  }

  return {
    menu,
    editingNodeId,
    editingEdgeIndex,
    openPaneMenu,
    openNodeMenu,
    openEdgeMenu,
    closeMenu,
    openNodeForm,
    closeNodeForm,
    openEdgeForm,
    closeEdgeForm,
    reset,
  };
}
