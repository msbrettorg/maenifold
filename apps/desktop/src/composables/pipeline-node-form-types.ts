// Form value types extracted to be shared between the field refs composable
// and the draft builders without circular imports.
import type { PipelineNodeAgentConfig } from '../lib/pipeline-types-node';

export interface PipelineNodeFormValues {
  workflowId: string;
  workflowAgent: string;
  workflowAgentConfig: PipelineNodeAgentConfig;
  workflowPrompt: string;
  promptAgent: string;
  promptAgentConfig: PipelineNodeAgentConfig;
  promptTemplate: string;
  thinkingTopic: string;
  thinkingAgent: string;
  thinkingAgentConfig: PipelineNodeAgentConfig;
  thinkingTotalThoughts: string;
  thinkingAnalysisType: string;
  toolName: string;
  toolAgent: string;
  toolAgentConfig: PipelineNodeAgentConfig;
  toolArgs: Record<string, unknown>;
  toolArgsValid: boolean;
  cwd?: string;
}

export const DEFAULT_PIPELINE_NODE_THOUGHTS = '5';
