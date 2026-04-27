// Async graph + concept actions for the knowledge store. Extracted from
// stores/knowledge.ts to keep modules under 300 LOC.
import type { Ref } from 'vue';
import type { useMemoryApi } from '../composables/useMemoryApi';
import { useMcpResources } from '../composables/useMcpResources';
import type { ResourceContent } from '../composables/mcpTransport';
import type { CommunityGraph, ConceptGraph } from '../lib/graph-types';
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
  activeTabUri: Ref<string | null>;
}

export interface KnowledgeGraphActions {
  loadGraphCommunities: () => Promise<void>;
  loadGraphConcepts: (communityId: number) => Promise<void>;
  focusCommunity: (communityId: number) => Promise<void>;
  showCommunityGraph: () => void;
  openConcept: (conceptName: string) => Promise<void>;
}

function parseResourceJson<T>(content: ResourceContent): T {
  if (typeof content.text !== 'string') {
    throw new Error(`Missing text payload for resource: ${content.uri}`);
  }

  try {
    return JSON.parse(content.text) as T;
  } catch (parseError) {
    const message = parseError instanceof Error ? parseError.message : String(parseError);
    throw new Error(`Invalid JSON payload for resource ${content.uri}: ${message}`);
  }
}

export function createKnowledgeGraphActions(
  refs: KnowledgeGraphRefs,
  memoryApi: ReturnType<typeof useMemoryApi>,
  openNote: (uri: string) => void,
): KnowledgeGraphActions {
  const mcpResources = useMcpResources();
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
    activeTabUri,
  } = refs;

  async function loadGraphCommunities(): Promise<void> {
    graphLoading.value = true;
    graphError.value = null;

    try {
      const resource = await mcpResources.readResource('graph://corpus/communities');
      communityGraph.value = parseResourceJson<CommunityGraph>(resource);
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
      const resource = await mcpResources.readResource(
        `graph://community/${encodeURIComponent(String(communityId))}`,
      );
      const graph = parseResourceJson<ConceptGraph>(resource);
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
        openNote(exactMatch.uri);
        return;
      }

      const anchorUri = findBestAnchorUri(normalized, files);
      if (anchorUri) {
        openNote(anchorUri);
        return;
      }

      const fallbackSearch = await memoryApi.searchMemories(normalized, { pageSize: 1 });
      const firstResult = fallbackSearch.results[0];
      if (firstResult) {
        openNote(firstResult.uri);
        return;
      }

      activeTabUri.value = null;
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
