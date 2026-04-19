// Watcher wiring for ComposerCanvas. Extracted from useComposerCanvas.ts
// to keep the main composable under 300 LOC.
import { nextTick, watch } from 'vue';
import type { Ref } from 'vue';
import { toPipelineNodeFromViewNode } from '../lib/pipeline-composer-flow';
import {
  DEFAULT_DIRECTION,
  hasViewStructureChanged,
  loadDirectionPreference,
  saveDirectionPreference,
  type Direction,
} from '../lib/composer-canvas-helpers';
import type { PipelineDefinition, PipelineView } from '../lib/pipeline-types';

export interface ComposerCanvasWatchersOptions {
  designMode: () => boolean;
  localDefinition: () => PipelineDefinition | undefined;
  view: () => PipelineView | null | undefined;
  layoutDirection: Ref<Direction>;
  fittedPipelineId: Ref<string | null>;
  fitView: (options?: { padding?: number; duration?: number }) => void;
  syncCanvas: (
    definition: PipelineDefinition,
    view?: PipelineView | null,
    layoutOptions?: { forceLayout?: boolean; skipLayout?: boolean },
  ) => Promise<void>;
  shouldSyncCanvasImmediately: (
    definition: PipelineDefinition,
    previousDefinition?: PipelineDefinition,
  ) => boolean;
  clearPendingDefinitionSync: () => void;
  scheduleDefinitionSync: (
    definition: PipelineDefinition,
    view?: PipelineView | null,
  ) => void;
}

export function setupComposerCanvasWatchers(opts: ComposerCanvasWatchersOptions): void {
  watch(
    () => opts.view()?.pipelineId ?? null,
    (pipelineSessionId) => {
      if (!pipelineSessionId) {
        opts.layoutDirection.value = DEFAULT_DIRECTION;
        return;
      }
      opts.layoutDirection.value = loadDirectionPreference(pipelineSessionId);
    },
    { immediate: true },
  );

  watch(opts.layoutDirection, (direction) => {
    const pipelineSessionId = opts.view()?.pipelineId;
    if (!pipelineSessionId) return;
    saveDirectionPreference(pipelineSessionId, direction);
  });

  watch(
    opts.localDefinition,
    async (definition, previousDefinition) => {
      if (!definition || !opts.designMode()) return;
      if (opts.shouldSyncCanvasImmediately(definition, previousDefinition)) {
        opts.clearPendingDefinitionSync();
        await opts.syncCanvas(definition, opts.view());
        nextTick(() => opts.fitView());
        return;
      }
      opts.scheduleDefinitionSync(definition, opts.view());
    },
    { immediate: true },
  );

  watch(
    opts.designMode,
    (designMode) => {
      if (designMode) opts.fittedPipelineId.value = null;
    },
    { immediate: true },
  );

  watch(
    () => opts.view()?.pipelineId ?? null,
    (pipelineId, previousPipelineId) => {
      if (pipelineId !== previousPipelineId) opts.fittedPipelineId.value = null;
    },
  );

  watch(
    opts.view,
    async (view, previousView) => {
      if (!view || opts.designMode()) return;
      const structureChanged = hasViewStructureChanged(view, previousView);
      await opts.syncCanvas(
        {
          nodes: view.nodes.map(toPipelineNodeFromViewNode),
          edges: view.edges,
        },
        view,
        {
          forceLayout: structureChanged,
          skipLayout: !structureChanged,
        },
      );
      const loadedPipelineId = view.pipelineId ?? null;
      if (loadedPipelineId && loadedPipelineId !== opts.fittedPipelineId.value) {
        opts.fittedPipelineId.value = loadedPipelineId;
        nextTick(() => opts.fitView({ padding: 0.15, duration: 200 }));
      }
    },
    { immediate: true },
  );
}
