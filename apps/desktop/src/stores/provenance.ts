import { computed, ref } from 'vue';
import { defineStore } from 'pinia';
import { PROVENANCE_ORDER, type ProvenanceKind, type ProvenanceLink } from '../lib/provenance';

function normalizeId(value: string | null | undefined): string | null {
  if (typeof value !== 'string') {
    return null;
  }

  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : null;
}

export const useProvenanceStore = defineStore('provenance', () => {
  const pendingGateTaskId = ref<string | null>(null);
  const pendingThinkingSelection = ref<{ thinkingSessionId: string; pipelineId: string | null } | null>(null);

  function buildLineageLinks(
    lineage: Partial<Record<ProvenanceKind, string | null | undefined>> & { parentWorkflowId?: string | null | undefined },
    labels: Partial<Record<ProvenanceKind, string | null | undefined>> = {},
  ): ProvenanceLink[] {
    const normalizedLineage: Partial<Record<ProvenanceKind, string>> = {};
    for (const kind of PROVENANCE_ORDER) {
      const normalized = normalizeId(lineage[kind]);
      if (normalized) {
        normalizedLineage[kind] = normalized;
      }
    }

    const normalizedParentWorkflowId = normalizeId(lineage.parentWorkflowId);

    return PROVENANCE_ORDER.flatMap((kind) => {
      const id = normalizedLineage[kind];
      if (!id) {
        return [];
      }

      const label = normalizeId(labels[kind]);
      return [{
        kind,
        id,
        ...(label ? { label } : {}),
        lineage: normalizedLineage,
        ...(normalizedParentWorkflowId ? { parentWorkflowId: normalizedParentWorkflowId } : {}),
      }];
    });
  }

  const pendingThinkingSessionId = computed(() => pendingThinkingSelection.value?.thinkingSessionId ?? null);
  const pendingThinkingPipelineId = computed(() => pendingThinkingSelection.value?.pipelineId ?? null);

  function queueGateTaskSelection(id: string | null | undefined): void {
    pendingGateTaskId.value = normalizeId(id);
  }

  function clearQueuedGateTaskSelection(): void {
    pendingGateTaskId.value = null;
  }

  function queueThinkingSessionSelection(id: string | null | undefined, pipelineId?: string | null | undefined): void {
    const normalizedThinkingSessionId = normalizeId(id);
    if (!normalizedThinkingSessionId) {
      pendingThinkingSelection.value = null;
      return;
    }

    pendingThinkingSelection.value = {
      thinkingSessionId: normalizedThinkingSessionId,
      pipelineId: normalizeId(pipelineId),
    };
  }

  function clearQueuedThinkingSessionSelection(): void {
    pendingThinkingSelection.value = null;
  }

  return {
    pendingGateTaskId,
    pendingThinkingSelection,
    pendingThinkingSessionId,
    pendingThinkingPipelineId,
    buildLineageLinks,
    queueGateTaskSelection,
    clearQueuedGateTaskSelection,
    queueThinkingSessionSelection,
    clearQueuedThinkingSessionSelection,
  };
});
