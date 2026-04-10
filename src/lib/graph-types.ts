export interface CommunityNode {
  id: number;
  anchor: string;
  size: number;
}

export interface CommunityEdge {
  source: number;
  target: number;
  weight: number;
}

export interface CommunityGraph {
  nodes: CommunityNode[];
  edges: CommunityEdge[];
}

export interface ConceptNode {
  name: string;
  occurrences: number;
}

export interface ConceptEdge {
  source: string;
  target: string;
  weight: number;
}

export interface ConceptGraph {
  communityId: number;
  anchor: string;
  nodes: ConceptNode[];
  edges: ConceptEdge[];
}

export interface ForceNode {
  id: string;
  label: string;
  radius: number;
  color: string;
  x?: number;
  y?: number;
  vx?: number;
  vy?: number;
  fx?: number | null;
  fy?: number | null;
}

export interface ForceEdge {
  source: string | ForceNode;
  target: string | ForceNode;
  weight: number;
}
