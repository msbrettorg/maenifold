import type { Ref } from 'vue';
import type { useMemoryApi } from '../composables/useMemoryApi';
import { extractConceptCandidates } from '../lib/knowledge-helpers';
import type { MemoryFile, SimilarConceptItem } from '../lib/workflow-types';

type MemoryApi = ReturnType<typeof useMemoryApi>;

interface RelatedConceptState {
  relatedConcepts: Ref<SimilarConceptItem[]>;
  relatedConceptsSource: Ref<string | null>;
  relatedConceptsLoading: Ref<boolean>;
  relatedConceptsError: Ref<string | null>;
}

export function createKnowledgeRelatedActions(state: RelatedConceptState, memoryApi: MemoryApi) {
  async function loadRelatedConceptsForNote(note: MemoryFile | null): Promise<void> {
    state.relatedConceptsSource.value = null;
    state.relatedConceptsError.value = null;
    state.relatedConcepts.value = [];

    const [seedConcept] = extractConceptCandidates(note);
    if (!seedConcept) return;

    state.relatedConceptsSource.value = seedConcept;
    state.relatedConceptsLoading.value = true;
    try {
      const result = await memoryApi.findSimilarConcepts(seedConcept, 8);
      state.relatedConcepts.value = result.results.filter(
        (item) => item.name.toLowerCase() !== seedConcept.toLowerCase(),
      );
    } catch (errorValue) {
      state.relatedConceptsError.value =
        errorValue instanceof Error ? errorValue.message : String(errorValue);
      state.relatedConcepts.value = [];
    } finally {
      state.relatedConceptsLoading.value = false;
    }
  }

  return { loadRelatedConceptsForNote };
}
