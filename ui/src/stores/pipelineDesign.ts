// Pipeline design store (T-600.4) — owns the editable pipeline definition for the
// Pipelines tab "design mode": node + edge CRUD, the dirty guard, the save-status
// badge state, and the lossless save round-trip through the Pipeline `update` op.
//
// Why a SEPARATE editable definition (not in-place edits of `selectedView`):
//   - The live runner (T-600.3) polls and mutates `selectedView`; editing it in
//     place would let a poll clobber unsaved work. The design graph is INDEPENDENT
//     state, seeded once on entry and on save, never on a poll tick.
//   - The canvas renders `designGraph` (this store) while design mode is active;
//     see `useCanvasSync`'s active-graph computed. Mutations here render live.
//
// editingSessionId is captured as the EXACT id used to load the pipeline
// (`selectedPipelineId`) and is the id passed to `updatePipeline` — never re-
// inferred from runtime fields.

import { computed, ref, watch } from 'vue';
import { defineStore } from 'pinia';
import type {
  PipelineDefinition,
  PipelineEdge,
  PipelineNode,
  PipelineStatus,
  PipelineView,
} from '../lib/pipeline-types';
import {
  definitionToView,
  viewToDefinition,
} from '../lib/pipelineDefinitionProjection';
import {
  constructNode,
  duplicateNode,
  uniqueNodeId,
  validateDefinition,
  type AddNodeKind,
  type ValidationIssue,
} from '../lib/pipelineDesignOps';
import { loadPositions, mergePosition, type XY } from '../components/pipeline/canvasBridge';
import { usePipelinesStore } from './pipelines';
import { useAppShellStore } from './appShell';

export type SaveStatus = 'idle' | 'unsaved' | 'saving' | 'saved' | 'error';

const DISCARD_PROMPT = 'You have unsaved pipeline design changes. Discard them?';
const DUPLICATE_OFFSET = 48;

function toMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export const usePipelineDesignStore = defineStore('pipelineDesign', () => {
  const pipelines = usePipelinesStore();
  const shell = useAppShellStore();

  const localDefinition = ref<PipelineDefinition | null>(null);
  const editingSessionId = ref<string | null>(null);
  const editingPipelineId = ref<string | null>(null);
  const editingName = ref<string | null>(null);
  const editingDescription = ref<string | null>(null);
  const editingStatus = ref<PipelineStatus>('pending');

  const dirty = ref(false);
  const saveStatus = ref<SaveStatus>('idle');
  const saveError = ref<string | null>(null);

  const designModeActive = computed(() => localDefinition.value !== null);

  const validationIssues = computed<ValidationIssue[]>(() =>
    localDefinition.value ? validateDefinition(localDefinition.value) : []
  );
  const canSave = computed(() => dirty.value && validationIssues.value.length === 0);

  // The PipelineView the canvas renders while design mode is active. Identity
  // (pipelineId) is preserved so the position cache + fit latch stay stable.
  const designGraph = computed<PipelineView | null>(() => {
    const def = localDefinition.value;
    const pipelineId = editingPipelineId.value;
    if (!def || !pipelineId) return null;
    return definitionToView(def, {
      pipelineId,
      name: editingName.value,
      description: editingDescription.value,
      status: editingStatus.value,
    });
  });

  function markDirty(): void {
    dirty.value = true;
    saveStatus.value = 'unsaved';
    saveError.value = null;
  }

  function setDefinition(next: PipelineDefinition): void {
    localDefinition.value = next;
  }

  // --- seeding / lifecycle --------------------------------------------------

  function seedFromView(view: PipelineView, sessionId: string): void {
    setDefinition(viewToDefinition(view));
    editingSessionId.value = sessionId;
    editingPipelineId.value = view.pipelineId ?? sessionId;
    editingName.value = view.name ?? null;
    editingDescription.value = view.description ?? null;
    editingStatus.value = view.status;
    dirty.value = false;
    saveStatus.value = 'saved';
    saveError.value = null;
  }

  // Enter design mode for the currently selected pipeline. Captures the exact
  // load-time session id. No-op without a selected view.
  function enterDesignMode(): boolean {
    const view = pipelines.selectedView;
    const sessionId = pipelines.selectedPipelineId;
    if (!view || !sessionId) return false;
    seedFromView(view, sessionId);
    saveStatus.value = 'idle';
    return true;
  }

  function exitDesignMode(): void {
    localDefinition.value = null;
    editingSessionId.value = null;
    editingPipelineId.value = null;
    editingName.value = null;
    editingDescription.value = null;
    dirty.value = false;
    saveStatus.value = 'idle';
    saveError.value = null;
  }

  // Revert unsaved edits back to the loaded view (used when the user confirms a
  // discard). Stays in design mode with a clean definition.
  function revertToLoaded(): void {
    const view = pipelines.selectedView;
    const sessionId = editingSessionId.value;
    if (view && sessionId) seedFromView(view, sessionId);
  }

  // --- node mutations -------------------------------------------------------

  function addNode(kind: AddNodeKind, flowPos?: XY): string | null {
    const def = localDefinition.value;
    if (!def) return null;
    const id = uniqueNodeId(def.nodes);
    const node = constructNode(kind, id);
    if (flowPos) mergePosition(editingPipelineId.value, id, flowPos);
    setDefinition({ ...def, nodes: [...def.nodes, node] });
    markDirty();
    return id;
  }

  function duplicateNodeById(id: string): string | null {
    const def = localDefinition.value;
    if (!def) return null;
    const original = def.nodes.find((node) => node.id === id);
    if (!original) return null;
    const newId = uniqueNodeId(def.nodes);
    const clone = duplicateNode(original, newId);
    const cached = loadPositions(editingPipelineId.value)[id];
    if (cached) {
      mergePosition(editingPipelineId.value, newId, {
        x: cached.x + DUPLICATE_OFFSET,
        y: cached.y + DUPLICATE_OFFSET,
      });
    }
    // Incident edges are intentionally NOT copied for a duplicate.
    setDefinition({ ...def, nodes: [...def.nodes, clone] });
    markDirty();
    return newId;
  }

  function replaceNode(id: string, node: PipelineNode): void {
    const def = localDefinition.value;
    if (!def) return;
    setDefinition({
      ...def,
      nodes: def.nodes.map((existing) => (existing.id === id ? node : existing)),
    });
    markDirty();
  }

  function deleteNodeById(id: string): void {
    const def = localDefinition.value;
    if (!def) return;
    setDefinition({
      ...def,
      nodes: def.nodes.filter((node) => node.id !== id),
      edges: def.edges.filter((edge) => edge.source !== id && edge.target !== id),
    });
    markDirty();
  }

  function disconnectNodeEdges(id: string): void {
    const def = localDefinition.value;
    if (!def) return;
    setDefinition({
      ...def,
      edges: def.edges.filter((edge) => edge.source !== id && edge.target !== id),
    });
    markDirty();
  }

  // --- edge mutations -------------------------------------------------------

  // Draw-to-connect: create an immediately-valid edge and return its index so
  // the caller can open the edge form. Self-edges are rejected (return null).
  function addEdge(source: string, target: string): number | null {
    const def = localDefinition.value;
    if (!def || source === target) return null;
    const edge: PipelineEdge = { source, target, type: 'success', mode: 'none' };
    const nextEdges = [...def.edges, edge];
    setDefinition({ ...def, edges: nextEdges });
    markDirty();
    return nextEdges.length - 1;
  }

  function replaceEdge(index: number, edge: PipelineEdge): void {
    const def = localDefinition.value;
    if (!def || index < 0 || index >= def.edges.length) return;
    setDefinition({
      ...def,
      edges: def.edges.map((existing, i) => (i === index ? edge : existing)),
    });
    markDirty();
  }

  function deleteEdgeByIndex(index: number): void {
    const def = localDefinition.value;
    if (!def || index < 0 || index >= def.edges.length) return;
    setDefinition({ ...def, edges: def.edges.filter((_, i) => i !== index) });
    markDirty();
  }

  // --- save -----------------------------------------------------------------

  async function save(): Promise<boolean> {
    const def = localDefinition.value;
    const sessionId = editingSessionId.value;
    if (!def || !sessionId) return false;
    if (validationIssues.value.length > 0) {
      saveStatus.value = 'error';
      saveError.value = `Cannot save: ${validationIssues.value.length} validation issue(s).`;
      return false;
    }
    saveStatus.value = 'saving';
    saveError.value = null;
    try {
      await pipelines.updatePipeline(sessionId, def, {
        name: editingName.value ?? undefined,
        description: editingDescription.value ?? undefined,
      });
      // Re-read the persisted state and apply it to the selection so the canvas
      // + inspector reflect exactly what the daemon stored.
      const fresh = await pipelines.viewPipeline(sessionId);
      seedFromView(fresh, sessionId);
      dirty.value = false;
      saveStatus.value = 'saved';
      saveError.value = null;
      return true;
    } catch (error) {
      saveStatus.value = 'error';
      saveError.value = toMessage(error);
      return false;
    }
  }

  // --- dirty guard ----------------------------------------------------------

  // Returns true if it is safe to navigate away (no design changes, or the user
  // confirmed discarding them). On a confirmed discard the edits are reverted so
  // a later return to the tab shows clean state.
  function confirmDiscardDesignChanges(): boolean {
    if (!designModeActive.value || !dirty.value) return true;
    const confirmFn = globalThis.confirm;
    const ok = typeof confirmFn === 'function' ? confirmFn(DISCARD_PROMPT) : true;
    if (ok) revertToLoaded();
    return ok;
  }

  // beforeunload covers window CLOSE only (not in-app tab/selection nav — those
  // route through confirmDiscardDesignChanges). Attached only while dirty.
  function handleBeforeUnload(event: BeforeUnloadEvent): void {
    event.preventDefault();
    event.returnValue = '';
  }

  watch(dirty, (isDirty) => {
    if (typeof window === 'undefined') return;
    if (isDirty) window.addEventListener('beforeunload', handleBeforeUnload);
    else window.removeEventListener('beforeunload', handleBeforeUnload);
  });

  // Integrate with the app tab store: while design mode is active, any tab
  // switch is gated by the discard guard (the app shell uses a tab store, not
  // vue-router). Registered once; the guard self-noops when not dirty.
  watch(designModeActive, (active) => {
    if (active) shell.registerNavigationGuard(confirmDiscardDesignChanges);
    else shell.clearNavigationGuard();
  });

  return {
    localDefinition,
    editingSessionId,
    editingPipelineId,
    editingName,
    editingDescription,
    editingStatus,
    dirty,
    saveStatus,
    saveError,
    designModeActive,
    validationIssues,
    canSave,
    designGraph,
    enterDesignMode,
    exitDesignMode,
    revertToLoaded,
    addNode,
    duplicateNodeById,
    replaceNode,
    deleteNodeById,
    disconnectNodeEdges,
    addEdge,
    replaceEdge,
    deleteEdgeByIndex,
    save,
    confirmDiscardDesignChanges,
  };
});
