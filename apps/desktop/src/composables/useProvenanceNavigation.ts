import { useAppShellStore } from '../stores/app-shell';
import { useKnowledgeStore } from '../stores/knowledge';
import { usePipelineStore } from '../stores/pipeline';
import { useProvenanceStore } from '../stores/provenance';
import type { ProvenanceLink } from '../lib/provenance';

export function useProvenanceNavigation() {
  const appShellStore = useAppShellStore();
  const pipelineStore = usePipelineStore();
  const knowledgeStore = useKnowledgeStore();
  const provenanceStore = useProvenanceStore();

  async function navigateTo(link: ProvenanceLink): Promise<void> {
    switch (link.kind) {
      case 'gate': {
        appShellStore.setActiveTab('gate');
        provenanceStore.queueGateTaskSelection(link.id);
        break;
      }
      case 'pipeline': {
        appShellStore.setActiveTab('pipelines');
        if (pipelineStore.currentPipelineId !== link.id) {
          await pipelineStore.loadPipeline(link.id);
        }
        break;
      }
      case 'thinking': {
        appShellStore.setActiveTab('pipelines');
        const owningPipelineId = link.lineage?.pipeline ?? link.parentWorkflowId ?? null;
        if (owningPipelineId && pipelineStore.currentPipelineId !== owningPipelineId) {
          await pipelineStore.loadPipeline(owningPipelineId);
        }
        provenanceStore.queueThinkingSessionSelection(link.id, owningPipelineId);
        break;
      }
      case 'knowledge': {
        appShellStore.setActiveTab('knowledge');
        if (link.id.startsWith('memory://')) {
          await knowledgeStore.openNote(link.id);
        }
        break;
      }
    }
  }

  return {
    navigateTo,
  };
}
