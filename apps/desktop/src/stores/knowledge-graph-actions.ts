// Async graph + concept actions for the knowledge store. Extracted from
// stores/knowledge.ts to keep modules under 300 LOC.
import type { Ref } from 'vue';
import type { useMemoryApi } from '../composables/useMemoryApi';
import { getDaemonUrl } from '../lib/daemon-url';
import type { CommunityGraph, ConceptGraph } from '../lib/graph-types';
import type { MemoryFile } from '../lib/workflow-types';
import {
  findBestAnchorUri,
  normalizeConceptName,
  type KnowledgeConceptNode,
} from '../lib/knowledge-helpers';

export interface KnowledgeGraphRefs {
  graphLoading: Ref<boolean>;
  graphError: Ref<string | null>;
  communityGraph: Ref<CommunityGraph | null>;
  conceptGraph: Ref<ConceptGraph | null>;
  conceptNodes: Ref<KnowledgeConceptNode[]>;
  activeCommunityId: Ref<number | null>;
  selectedConcept: Ref<string | null>;
  conceptContextLoading: Ref<boolean>;
  conceptContextError: Ref<string | null>;
  activeNote: Ref<MemoryFile | null>;
  activeUri: Ref<string | null>;
}

export interface KnowledgeGraphActions {
  loadGraphCommunities: () => Promise<void>;
  loadGraphConcepts: (communityId: number) => Promise<void>;
  focusCommunity: (communityId: number) => Promise<void>;
  showCommunityGraph: () => void;
  openConcept: (conceptName: string) => Promise<void>;
}

export function createKnowledgeGraphActions(
  refs: KnowledgeGraphRefs,
  memoryApi: ReturnType<typeof useMemoryApi>,
  openNote: (uri: string) => Promise<void>,
): KnowledgeGraphActions {
  const {
    graphLoading,
    graphError,
    communityGraph,
    conceptGraph,
    conceptNodes,
    activeCommunityId,
    selectedConcept,
    conceptContextLoading,
    conceptContextError,
    activeNote,
    activeUri,
  } = refs;

  async function loadGraphCommunities(): Promise<void> {
    graphLoading.value = true;
    graphError.value = null;

    try {
      const response = await fetch(getDaemonUrl('/api/graph/communities'));
      if (!response.ok) {
        throw new Error(
          (await response.text()) || `Failed to load graph communities (${response.status}).`,
        );
      }

      communityGraph.value = (await response.json()) as CommunityGraph;
    } catch (errorValue) {
      graphError.value = errorValue instanceof Error ? errorValue.message : String(errorValue);
      communityGraph.value = null;
    } finally {
      graphLoading.value = false;
    }
  }

  async function loadGraphConcepts(communityId: number): Promise<void> {
    graphLoading.value = true;
    graphError.value = null;

    try {
      const response = await fetch(getDaemonUrl(`/api/graph/communities/${communityId}/concepts`));
      if (!response.ok) {
        throw new Error(
          (await response.text()) || `Failed to load concepts (${response.status}).`,
        );
      }

      const graph = (await response.json()) as ConceptGraph;
      const edgeCounts = new Map<string, number>();
      for (const edge of graph.edges) {
        edgeCounts.set(edge.source, (edgeCounts.get(edge.source) ?? 0) + 1);
        edgeCounts.set(edge.target, (edgeCounts.get(edge.target) ?? 0) + 1);
      }

      conceptGraph.value = graph;
      conceptNodes.value = graph.nodes
        .map((node) => ({
          name: node.name,
          occurrences: node.occurrences,
          edgeCount: edgeCounts.get(node.name) ?? 0,
        }))
        .sort((left, right) => {
          if (right.edgeCount !== left.edgeCount) {
            return right.edgeCount - left.edgeCount;
          }
          return left.name.localeCompare(right.name);
        });
      activeCommunityId.value = communityId;
    } catch (errorValue) {
      graphError.value = errorValue instanceof Error ? errorValue.message : String(errorValue);
      conceptGraph.value = null;
      conceptNodes.value = [];
    } finally {
      graphLoading.value = false;
    }
  }

  async function focusCommunity(communityId: number): Promise<void> {
    await loadGraphConcepts(communityId);
  }

  function showCommunityGraph(): void {
    activeCommunityId.value = null;
    conceptGraph.value = null;
    conceptNodes.value = [];
    selectedConcept.value = null;
    conceptContextError.value = null;
  }

  async function openConcept(conceptName: string): Promise<void> {
    const normalized = normalizeConceptName(conceptName);
    if (!normalized) return;

    selectedConcept.value = normalized;
    conceptContextLoading.value = true;
    conceptContextError.value = null;

    try {
      const context = await memoryApi.buildContext(normalized, { depth: 1, maxEntities: 30 });

      if (typeof context.communityId === 'number') {
        activeCommunityId.value = context.communityId;
      }

      const files = [
        ...context.directRelations.flatMap((relation) => relation.files),
        ...context.expandedRelations.flatMap((relation) => relation.files),
      ];

      const exactResults = await memoryApi.searchMemories(normalized, { mode: 'FullText' });
      const exactMatch = exactResults.results.find(
        (result) => result.title?.toLowerCase() === normalized.toLowerCase(),
      );
      if (exactMatch?.uri) {
        await openNote(exactMatch.uri);
        return;
      }

      const anchorUri = findBestAnchorUri(normalized, files);
      if (anchorUri) {
        await openNote(anchorUri);
        return;
      }

      const fallbackSearch = await memoryApi.searchMemories(normalized, { pageSize: 1 });
      const firstResult = fallbackSearch.results[0];
      if (firstResult) {
        await openNote(firstResult.uri);
        return;
      }

      activeNote.value = null;
      activeUri.value = null;
      conceptContextError.value = `No anchor note found for ${normalized}`;
    } catch (errorValue) {
      conceptContextError.value =
        errorValue instanceof Error ? errorValue.message : String(errorValue);
    } finally {
      conceptContextLoading.value = false;
    }
  }

  return {
    loadGraphCommunities,
    loadGraphConcepts,
    focusCommunity,
    showCommunityGraph,
    openConcept,
  };
}
