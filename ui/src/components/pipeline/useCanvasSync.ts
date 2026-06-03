// useCanvasSync — guarded bidirectional Pinia ⇄ Vue Flow synchronization.
//
// THE LOOP WE ARE PREVENTING:
//   store.selectedView changes → applyStoreToFlow() calls setNodes/setEdges →
//   Vue Flow emits nodes-change (dimensions/position/select) → a change handler
//   writes back → store mutates → watcher fires applyStoreToFlow() → … forever.
//
// THE GUARD: a single `syncing` boolean. `applyStoreToFlow()` raises it before
// it touches the Vue Flow store and lowers it on the next tick (after Vue Flow
// has finished emitting its initial change burst). Every flow→store/persistence
// handler early-returns while `syncing` is true. This is the same discipline the
// old UI spread across useComposerSync/useComposerCanvas*, distilled to one flag.
//
// Position survival on graft: structural re-applies pass already-cached
// coordinates as fixed; cached positions always win over freshly-computed ones,
// so only genuinely new nodes are laid out and existing nodes never jump.

import { nextTick, ref, watch } from 'vue';
import { storeToRefs } from 'pinia';
import { useVueFlow } from '@vue-flow/core';
import type { Node } from '@vue-flow/core';
import { usePipelinesStore } from '../../stores/pipelines';
import { useCanvasController } from './useCanvasController';
import { useCanvasLayout } from './useCanvasLayout';
import {
  fallbackGrid,
  loadDirection,
  loadPositions,
  savePositions,
  saveDirection,
  viewToFlowEdges,
  viewToFlowNodes,
  type FlowNodeData,
  type XY,
} from './canvasBridge';

export function useCanvasSync(flowId: string) {
  const store = usePipelinesStore();
  const { selectedView } = storeToRefs(store);
  const controller = useCanvasController();
  const { layout } = useCanvasLayout();
  const vf = useVueFlow(flowId);

  // The single reactivity-loop guard. Exposed so the canvas can show a
  // brief "laying out…" affordance and so tests can assert on it.
  const syncing = ref(false);
  const layingOut = ref(false);
  const fittedPipelineId = ref<string | null>(null);
  // Mid-poll-drag protection: while a node is being dragged we never call
  // setNodes (which would yank the dragged node back and could re-enter the
  // sync loop). A store change that arrives mid-drag is remembered and replayed
  // once the drag finishes.
  const dragging = ref(false);
  const pendingReapply = ref(false);
  // Latest-wins guard. `applyStoreToFlow` is async (awaits ELK + nextTick); a
  // newer invocation (e.g. the next poll tick) must win. Each run captures a
  // monotonic token and bails after every await if a newer run has started,
  // so a stale older run can never land its setNodes/persist last.
  let applyGen = 0;
  // Distinguishes a navigation-time direction RESTORE (adopting the newly
  // selected pipeline's saved orientation) from an explicit USER toggle of the
  // LR/TB control. A restore must NOT force a relayout — that would discard the
  // pipeline's cached drag positions; the selectedView watcher's non-forced
  // apply already honors the cache. A user toggle must still force a relayout.
  let restoringDirection = false;

  function currentPipelineId(): string | null {
    return selectedView.value?.pipelineId ?? null;
  }

  /** Live position of a node already on the canvas, if any. */
  function livePosition(nodeId: string): XY | undefined {
    const existing = vf.getNodes.value.find((node) => node.id === nodeId);
    return existing ? { x: existing.position.x, y: existing.position.y } : undefined;
  }

  /**
   * Push the current store view into Vue Flow. Awaits ELK layout for any
   * uncached nodes BEFORE committing nodes, so positioned nodes render in one
   * frame (no unpositioned flash). `forceLayout` re-lays-out the whole graph
   * (used by the toolbar auto-layout button and on a direction switch).
   */
  async function applyStoreToFlow(forceLayout = false): Promise<void> {
    // Never mutate the Vue Flow node set mid-drag: setNodes would snap the
    // dragged node back to its stored coordinate and lose the gesture. Defer
    // and replay once the drag ends.
    if (dragging.value && !forceLayout) {
      pendingReapply.value = true;
      return;
    }

    // Claim this invocation's generation. Any run that starts after this point
    // increments applyGen, so our post-await checks below detect supersession.
    const myGen = ++applyGen;
    const view = selectedView.value;
    if (!view) {
      syncing.value = true;
      vf.setNodes([]);
      vf.setEdges([]);
      await nextTick();
      syncing.value = false;
      return;
    }

    const pipelineId = view.pipelineId;
    const cached = loadPositions(pipelineId);
    let nodes = viewToFlowNodes(view, cached);
    const edges = viewToFlowEdges(view.edges, controller.direction.value);

    const hasUncached = nodes.some((node) => !cached[node.id]);
    const shouldLayout = nodes.length > 0 && (forceLayout || hasUncached);

    if (shouldLayout) {
      layingOut.value = true;
      // On a forced re-layout we discard the cache so ELK owns every position;
      // otherwise we pin cached nodes and only new nodes get placed.
      const fixed = forceLayout ? {} : cached;
      const computed = await layout(nodes, edges, controller.direction.value, fixed);
      nodes = nodes.map((node, index) => {
        const position: XY = forceLayout
          ? computed[node.id] ?? cached[node.id] ?? fallbackGrid(index)
          : cached[node.id] ?? computed[node.id] ?? fallbackGrid(index);
        return { ...node, position };
      });
      layingOut.value = false;
    }

    // For incremental re-applies (status polls, grafts) prefer the live on-canvas
    // position of any node that already exists, so a re-projection never nudges a
    // node the user has hand-placed (or is between drag-end and persistence).
    if (!forceLayout) {
      nodes = nodes.map((node) => {
        const live = livePosition(node.id);
        return live ? { ...node, position: live } : node;
      });
    }

    // Bail-out A: sits after `await layout(...)`. If a newer invocation began
    // while ELK was resolving, abandon this stale projection before it can
    // clobber the canvas with out-of-date nodes/edges.
    if (myGen !== applyGen) return;

    // Drag re-check A: a drag may have STARTED during `await layout(...)`.
    // Writing now would snap the dragged node back to its stored coordinate, so
    // defer the structural apply — onNodeDragStop replays pendingReapply.
    // `syncing` is still false on this path, so nothing is left stuck.
    if (dragging.value && !forceLayout) {
      pendingReapply.value = true;
      layingOut.value = false;
      return;
    }

    syncing.value = true;
    vf.setNodes(nodes as Node[]);
    vf.setEdges(edges);
    await nextTick();
    syncing.value = false;

    // Bail-out B: sits after `await nextTick()`. If superseded during the tick
    // window, skip persistence/fit so an older run never overwrites the newer
    // run's saved coordinates or viewport.
    if (myGen !== applyGen) return;

    // Drag re-check B: a drag may have STARTED during `await nextTick()`. The
    // structural setNodes already landed (with up-to-date coordinates), but we
    // must not persist mid-flux positions or steal the viewport mid-drag.
    // onNodeDragStop will persist on release. `syncing` is already false here.
    if (dragging.value && !forceLayout) {
      pendingReapply.value = true;
      return;
    }

    // Persist resolved coordinates so a reload restores the same layout even
    // for nodes that were only ever auto-placed.
    savePositions(
      vf.getNodes.value.map((node) => ({ id: node.id, position: node.position })),
      pipelineId
    );

    if (pipelineId && pipelineId !== fittedPipelineId.value) {
      fittedPipelineId.value = pipelineId;
      void nextTick(() => vf.fitView({ padding: 0.18, duration: 220 }));
    }
  }

  // --- store → flow ---------------------------------------------------------

  // Pipeline identity changes: adopt that pipeline's saved layout direction and
  // reset the fit latch so the new graph gets framed.
  watch(
    () => currentPipelineId(),
    (pipelineId) => {
      const restored = pipelineId ? loadDirection(pipelineId) : 'LR';
      // Only flag + mutate when the direction will actually change, so the flag
      // cannot leak into a later genuine toggle when the restored direction
      // already equals the current one.
      if (restored !== controller.direction.value) {
        restoringDirection = true;
        controller.direction.value = restored;
      }
      fittedPipelineId.value = null;
    }
  );

  // Any view change (selection, status poll, graft) re-projects into the canvas.
  watch(selectedView, () => {
    void applyStoreToFlow();
  });

  // Direction switch: persist + full relayout in the new orientation — EXCEPT a
  // navigation-time restore, which must keep the restored pipeline's cached drag
  // positions (the non-forced selectedView apply handles its rendering).
  watch(controller.direction, (direction) => {
    if (restoringDirection) {
      restoringDirection = false;
      return;
    }
    const pipelineId = currentPipelineId();
    if (pipelineId) saveDirection(pipelineId, direction);
    void applyStoreToFlow(true);
  });

  // --- flow → persistence / selection (all guarded) -------------------------

  vf.onNodeDragStart(() => {
    dragging.value = true;
  });

  vf.onNodeDragStop(() => {
    dragging.value = false;
    if (!syncing.value) {
      savePositions(
        vf.getNodes.value.map((node) => ({ id: node.id, position: node.position })),
        currentPipelineId()
      );
    }
    // Replay any store change that arrived while the user was dragging.
    if (pendingReapply.value) {
      pendingReapply.value = false;
      void applyStoreToFlow();
    }
  });

  watch(
    () => vf.getSelectedNodes.value.length + vf.getSelectedEdges.value.length,
    (count) => {
      controller.hasSelection.value = count > 0;
    }
  );

  // --- imperative actions exposed to the teleported toolbar -----------------

  controller.registerActions({
    autoLayout: () => {
      void applyStoreToFlow(true);
    },
    fitView: () => {
      vf.fitView({ padding: 0.18, duration: 220 });
    },
    deleteSelected: () => {
      if (syncing.value) return;
      const nodeIds = vf.getSelectedNodes.value.map((node) => node.id);
      const edgeIds = vf.getSelectedEdges.value.map((edge) => edge.id);
      if (nodeIds.length) vf.removeNodes(nodeIds);
      if (edgeIds.length) vf.removeEdges(edgeIds);
      savePositions(
        vf.getNodes.value.map((node) => ({ id: node.id, position: node.position })),
        currentPipelineId()
      );
    },
  });

  function dispose(): void {
    controller.clearActions();
  }

  return { syncing, layingOut, applyStoreToFlow, dispose };
}

export type { FlowNodeData };
