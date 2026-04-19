import { computed, ref, type ComputedRef } from 'vue';
import type { ViewNode } from '../lib/pipeline-types';
import type { ProvenanceLink } from '../lib/provenance';
import { useProvenanceStore } from '../stores/provenance';

export type SelectedEdge = { source: string; target: string } | null;

export type UsePipelineViewStateOptions = {
  currentView: ComputedRef<{ pipelineId?: string; nodes: ViewNode[] } | null | undefined> | { value: { pipelineId?: string; nodes: ViewNode[] } | null | undefined };
  currentPipelineId: ComputedRef<string | null> | { value: string | null };
};

function firstNonEmptyId(...values: Array<string | null | undefined>): string | null {
  for (const value of values) {
    if (typeof value !== 'string') continue;
    const trimmed = value.trim();
    if (trimmed.length > 0) return trimmed;
  }
  return null;
}

export function usePipelineViewState(options: UsePipelineViewStateOptions) {
  const provenance = useProvenanceStore();
  const { currentView, currentPipelineId } = options;

  const selectedNodeId = ref<string | null>(null);
  const selectedEdge = ref<SelectedEdge>(null);
  const configModalNodeId = ref<string | null>(null);
  const configModalEdge = ref<SelectedEdge>(null);

  const isConfigModalOpen = computed(() => configModalNodeId.value !== null || configModalEdge.value !== null);
  const existingNodeIds = computed(() => currentView.value?.nodes.map((node) => node.nodeId) ?? []);

  const selectedNode = computed(() =>
    currentView.value?.nodes.find((node) => node.nodeId === selectedNodeId.value) ?? null);
  const selectedNodeProvenanceDetail = computed(() => selectedNode.value?.provenance?.detail ?? null);

  const currentPipelineLineageId = computed(() =>
    firstNonEmptyId(
      selectedNodeProvenanceDetail.value?.lineagePipelineRunId,
      selectedNodeProvenanceDetail.value?.lineage_pipeline_run_id,
      currentView.value?.pipelineId,
      currentPipelineId.value,
    ));

  const pipelineLineageLinks = computed<ProvenanceLink[]>(() =>
    provenance.buildLineageLinks({
      gate: firstNonEmptyId(
        selectedNodeProvenanceDetail.value?.lineageGateTaskId,
        selectedNodeProvenanceDetail.value?.lineage_gate_task_id,
      ),
      pipeline: currentPipelineLineageId.value,
      thinking: firstNonEmptyId(
        selectedNodeProvenanceDetail.value?.lineageThinkingSessionId,
        selectedNodeProvenanceDetail.value?.lineage_thinking_session_id,
        selectedNode.value?.thinkingSessionId,
        selectedNodeProvenanceDetail.value?.thinkingSessionId,
      ),
      knowledge: firstNonEmptyId(
        selectedNodeProvenanceDetail.value?.lineageMemoryNoteId,
        selectedNodeProvenanceDetail.value?.lineage_memory_note_id,
      ),
      parentWorkflowId: firstNonEmptyId(
        selectedNodeProvenanceDetail.value?.parentWorkflowId,
        selectedNodeProvenanceDetail.value?.parent_workflow_id,
      ),
    }, {
      pipeline: currentView.value?.pipelineId ?? currentPipelineId.value ?? undefined,
    }));

  const provenanceLinks = computed(() =>
    pipelineLineageLinks.value
      .filter((link) => link.kind !== 'pipeline' && link.id !== currentPipelineLineageId.value));

  function handleNodeSelected(nodeId: string | null) {
    selectedNodeId.value = nodeId;
    if (nodeId) selectedEdge.value = null;
  }
  function handleEdgeSelected(edge: SelectedEdge) {
    selectedEdge.value = edge;
    if (edge) selectedNodeId.value = null;
  }
  function clearInspectorSelection() {
    selectedNodeId.value = null;
    selectedEdge.value = null;
  }
  function handleNodeDoubleClick(nodeId: string) {
    configModalNodeId.value = nodeId;
    configModalEdge.value = null;
  }
  function handleEdgeDoubleClick(edge: { source: string; target: string }) {
    configModalEdge.value = edge;
    configModalNodeId.value = null;
  }
  function closeConfigModal() {
    configModalNodeId.value = null;
    configModalEdge.value = null;
  }

  return {
    selectedNodeId,
    selectedEdge,
    selectedNode,
    configModalNodeId,
    configModalEdge,
    isConfigModalOpen,
    existingNodeIds,
    currentPipelineLineageId,
    pipelineLineageLinks,
    provenanceLinks,
    handleNodeSelected,
    handleEdgeSelected,
    clearInspectorSelection,
    handleNodeDoubleClick,
    handleEdgeDoubleClick,
    closeConfigModal,
  };
}
