export interface WorkflowSession {
  sessionId: string;
  uri: string;
  workflowId: string;
  workflowName: string;
  started: string;
  status: 'active' | 'completed' | 'abandoned';
  steps: WorkflowStep[];
  childWorkflowUri?: string;
}

export interface WorkflowStep {
  stepNumber: number;
  totalSteps: number;
  id: string;
  name: string;
  status: 'pending' | 'active' | 'completed';
  response?: string;
  respondedAt?: string;
  submachineSessionId?: string;
  requiresSequentialThinking: boolean;
}

export interface ThinkingSession {
  sessionId: string;
  uri: string;
  thoughts: Thought[];
  conclusion?: string;
  status: 'active' | 'completed' | 'abandoned';
}

export interface Thought {
  number: number;
  totalThoughts: number;
  agent: string;
  content: string;
  timestamp: string;
}

export type MemorySearchMode = 'Hybrid' | 'Semantic' | 'FullText';

export type RecentActivityFilter = 'thinking' | 'memory' | 'assumptions' | 'all';

export interface MemoryFile {
  uri: string;
  title: string;
  content: string;
  created: string | null;
  modified: string | null;
  lineageGateTaskId?: string | null;
  lineagePipelineRunId?: string | null;
  lineageThinkingSessionId?: string | null;
  lineageMemoryNoteId?: string | null;
  parentWorkflowId?: string | null;
  metadata?: Record<string, unknown>;
}

export interface MemorySearchItem {
  uri: string;
  title: string;
  snippet: string;
  score: number;
}

export interface SearchResult {
  results: MemorySearchItem[];
  totalCount: number;
  page: number;
  pageSize: number;
}

export interface RecentActivityItem {
  uri: string;
  title: string;
  modified: string;
  sections: string[];
}

export interface RecentResult {
  items: RecentActivityItem[];
}

export interface BuildContextRelation {
  name: string;
  coOccurrenceCount: number;
  files: string[];
  weightedScore: number | null;
  communityId: number | null;
}

export interface BuildContextResult {
  conceptName: string;
  depth: number;
  communityId: number | null;
  directRelations: BuildContextRelation[];
  expandedRelations: BuildContextRelation[];
}

export interface SimilarConceptItem {
  name: string;
  similarity: number;
  weightedSimilarity: number | null;
}

export interface SimilarConceptResult {
  conceptName: string;
  totalCount: number;
  results: SimilarConceptItem[];
}

export interface WorkflowTreeWorkflowNode {
  type: 'workflow';
  data: WorkflowSession;
  children: WorkflowTreeNode[];
}

export interface WorkflowTreeThinkingNode {
  type: 'thinking';
  data: ThinkingSession;
  children: WorkflowTreeNode[];
}

export type WorkflowTreeNode = WorkflowTreeWorkflowNode | WorkflowTreeThinkingNode;
