// Surgery-mode UI controller (T-600.5) — a module-level singleton, PARALLEL to
// (never shared with) the design controller. It decouples the Vue Flow canvas
// and the teleported surgery surfaces (context menu + graft/join/fork dialogs),
// which live in different DOM subtrees and so cannot talk via props/emits.
//
// Surgery operates on the SELECTED LIVE (server-backed) pipeline and mutates it
// server-side immediately — a DIFFERENT mode from design mode's local-definition
// editing. The surgery menu/toolbar are SUPPRESSED while design mode is active so
// the two overlays/controllers never both open.
//
// Poll coordination: the live runner registers its poll stop/start here so a
// surgery action (in useSurgeryActions) can pause polling around a mutation and
// resume it afterward — the same registration seam useCanvasController uses for
// the canvas's imperative actions.

import { ref } from 'vue';
import { usePipelinesStore } from '../../../stores/pipelines';

export type SurgeryDialogKind = 'graft' | 'join' | 'fork';

export interface SurgeryMenuState {
  // Screen (client) coordinates for fixed-positioning the floating menu.
  x: number;
  y: number;
  // The right-clicked live node id the menu targets.
  nodeId: string;
}

export interface SurgeryDialogState {
  kind: SurgeryDialogKind;
  // For fork/graft opened from a node: the node the operation seeds from.
  nodeId: string | null;
}

export interface PollControl {
  stop: () => void;
  start: () => void;
}

const menu = ref<SurgeryMenuState | null>(null);
const dialog = ref<SurgeryDialogState | null>(null);

// The pipeline (session) id the open menu/dialog was opened against, captured at
// OPEN time. useSurgeryActions compares this against the store's CURRENT
// selectedPipelineId before mutating and refuses if the user switched pipelines
// while a surgery surface was open (T-600.5 Defect 3 — cross-pipeline leak).
// Cleared by reset() on every pipeline switch and on close.
const targetSessionId = ref<string | null>(null);

// Shared action state, module-level so EVERY consumer (menu, toolbar, dialogs,
// and a status toast) sees the same busy/error/notice. useSurgeryActions writes
// these; components read them. (A per-call composable ref would not be shared.)
const busy = ref(false);
const lastError = ref<string | null>(null);
const notice = ref<string | null>(null);

let pollControl: PollControl | null = null;

export function useSurgeryController() {
  function openNodeMenu(x: number, y: number, nodeId: string): void {
    dialog.value = null;
    targetSessionId.value = usePipelinesStore().selectedPipelineId;
    menu.value = { x, y, nodeId };
  }

  function closeMenu(): void {
    menu.value = null;
  }

  function openDialog(kind: SurgeryDialogKind, nodeId: string | null = null): void {
    menu.value = null;
    targetSessionId.value = usePipelinesStore().selectedPipelineId;
    dialog.value = { kind, nodeId };
  }

  function closeDialog(): void {
    dialog.value = null;
  }

  function reset(): void {
    menu.value = null;
    dialog.value = null;
    targetSessionId.value = null;
  }

  // --- poll-control registration (runner ⇄ surgery actions) ----------------

  function registerPollControl(control: PollControl): void {
    pollControl = control;
  }

  function clearPollControl(): void {
    pollControl = null;
  }

  function pausePolling(): void {
    pollControl?.stop();
  }

  function resumePolling(): void {
    // The runner's `start` self-guards terminal status + design mode, so a
    // resume after a mutation only restarts polling when it should.
    pollControl?.start();
  }

  return {
    menu,
    dialog,
    targetSessionId,
    busy,
    lastError,
    notice,
    openNodeMenu,
    closeMenu,
    openDialog,
    closeDialog,
    reset,
    registerPollControl,
    clearPollControl,
    pausePolling,
    resumePolling,
  };
}
