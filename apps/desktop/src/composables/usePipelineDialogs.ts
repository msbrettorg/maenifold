import { ref } from 'vue';
import { storeToRefs } from 'pinia';
import { usePipelineStore } from '../stores/pipeline';
import type { PipelineEdge, PipelineNode } from '../lib/pipeline-types';

/**
 * Manages graft-nodes and join-pipeline dialog state and submit handlers for PipelineView.
 */
export function usePipelineDialogs() {
  const store = usePipelineStore();
  const { currentView, errorMessage } = storeToRefs(store);

  const graftDialogOpen = ref(false);
  const graftSubmitting = ref(false);
  const joinDialogOpen = ref(false);
  const joinSubmitting = ref(false);

  async function handleGraftSubmit(nodes: PipelineNode[], edges: PipelineEdge[]) {
    if (!currentView.value?.pipelineId || graftSubmitting.value) return;

    graftSubmitting.value = true;
    const pipelineId = currentView.value.pipelineId;

    try {
      await store.graftNodes(nodes, edges);
      if (errorMessage.value) return;
      await store.loadPipeline(pipelineId);
      if (errorMessage.value) return;
      graftDialogOpen.value = false;
    } finally {
      graftSubmitting.value = false;
    }
  }

  async function handleJoinSubmit(sourceSessionId: string, edges: PipelineEdge[]) {
    if (!currentView.value?.pipelineId || joinSubmitting.value) return;

    joinSubmitting.value = true;
    const pipelineId = currentView.value.pipelineId;

    try {
      await store.joinPipeline(pipelineId, sourceSessionId, edges);
      if (errorMessage.value) return;
      await store.loadPipeline(pipelineId);
      if (errorMessage.value) return;
      joinDialogOpen.value = false;
    } finally {
      joinSubmitting.value = false;
    }
  }

  return {
    graftDialogOpen,
    graftSubmitting,
    joinDialogOpen,
    joinSubmitting,
    handleGraftSubmit,
    handleJoinSubmit,
  };
}
