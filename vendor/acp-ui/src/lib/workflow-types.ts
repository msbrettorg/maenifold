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
