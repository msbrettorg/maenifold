import { onBeforeUnmount } from 'vue';

import type { PipelineDefinition, PipelineView } from '../lib/pipeline-types';

const DEBOUNCED_DEFINITION_SYNC_MS = 150;

type SyncCanvasFn = (definition: PipelineDefinition, view?: PipelineView | null) => void;

/**
 * Manages debounced canvas sync for PipelineComposer.
 * Handles the timer lifecycle including cleanup on unmount.
 */
export function useComposerSync(syncCanvas: SyncCanvasFn, getDesignMode: () => boolean) {
  let definitionSyncTimer: ReturnType<typeof setTimeout> | null = null;

  function clearPendingDefinitionSync(): void {
    if (definitionSyncTimer === null) return;
    globalThis.clearTimeout(definitionSyncTimer);
    definitionSyncTimer = null;
  }

  function scheduleDefinitionSync(definition: PipelineDefinition, view?: PipelineView | null): void {
    clearPendingDefinitionSync();
    definitionSyncTimer = globalThis.setTimeout(() => {
      definitionSyncTimer = null;
      if (!getDesignMode()) return;
      syncCanvas(definition, view);
    }, DEBOUNCED_DEFINITION_SYNC_MS);
  }

  function shouldSyncCanvasImmediately(
    definition: PipelineDefinition,
    previousDefinition?: PipelineDefinition,
  ): boolean {
    if (!previousDefinition) return true;
    return (
      definition.nodes.length !== previousDefinition.nodes.length
      || definition.edges.length !== previousDefinition.edges.length
    );
  }

  onBeforeUnmount(clearPendingDefinitionSync);

  return { clearPendingDefinitionSync, scheduleDefinitionSync, shouldSyncCanvasImmediately };
}
