// T-APP-001.2: TypeScript interfaces matching C# BuildContextResult
import type * as d3 from "d3-force";

export interface BuildContextResult {
  conceptName: string;
  depth: number;
  directRelations: RelatedConcept[];
  expandedRelations: string[];
  communityId: number | null;
  communitySiblings: CommunitySibling[];
}

export interface RelatedConcept {
  name: string;
  coOccurrenceCount: number;
  decayWeight: number;
  weightedScore: number;
  communityId: number | null;
  files: string[];
}

export interface CommunitySibling {
  name: string;
  communityId: number;
  sharedNeighborCount: number;
  normalizedOverlap: number;
}

export interface GraphNode extends d3.SimulationNodeDatum {
  id: string;
  communityId: number | null;
  weightedScore: number;
  expanded: boolean;
  isCenter: boolean;
}

export interface GraphEdge {
  source: string | GraphNode;
  target: string | GraphNode;
  weight: number;
}
