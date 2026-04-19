/**
 * Pipeline template registry — pure typed data.
 *
 * Each template declares an intent-first label, a human-readable description,
 * the raw PipelineDefinition with `{{variable}}` placeholders, a nodeLabels
 * authoring map that the runtime stamps onto instantiated nodes as
 * `displayLabel`, and optional capability requirements that gate availability.
 *
 * Templates are resolved at instantiation time by pipeline-template-runtime.ts.
 */
import type { PipelineDefinition } from './pipeline-types';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/** Structural variables the runtime must interpolate before creating the pipeline. */
export interface TemplateVariables {
  primaryAgent: string;
  criticAgent: string;
}

/** A tool that must be discovered via MCP for the template to be available. */
export interface TemplateCapabilityGate {
  /** Fully-qualified MCP tool name, e.g. "msx-mcp-get_account_overview". */
  toolName: string;
}

export type TemplateCategory = 'universal' | 'enterprise';

export interface TemplateInputDefinition {
  /** Key passed to MCP `Pipeline run` as `inputs[key]`. */
  key: string;
  /** Human-readable label shown in the run form. */
  label: string;
  /** Optional helper copy shown under the field label. */
  description?: string;
  /** Placeholder text for the runtime field. */
  placeholder?: string;
  /** Whether the run is blocked until a non-empty value is provided. */
  required?: boolean;
}

export interface PipelineTemplate {
  /** Unique slug used as registry key. */
  id: string;
  /** Intent-first label shown in the gallery. */
  label: string;
  /** One-line description of what this pipeline does. */
  description: string;
  /** Icon key for lucide-vue-next. */
  icon: string;
  category: TemplateCategory;
  /** When non-empty, ALL listed tools must be discovered for the template to appear. */
  capabilities: readonly TemplateCapabilityGate[];
  /** Raw pipeline definition with `{{variable}}` placeholders in string values. */
  definition: PipelineDefinition;
  /** Explicit runtime inputs shown only when the user runs the pipeline. */
  runInputs: readonly TemplateInputDefinition[];
  /** Map from node id → human-readable label. */
  nodeLabels: Readonly<Record<string, string>>;
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

export const PIPELINE_TEMPLATES: readonly PipelineTemplate[] = [
  // ── Universal templates ─────────────────────────────────────────────
  {
    id: 'research-and-verify',
    label: 'Research & Verify',
    description: 'Investigate a topic using the scientific method, then peer-review the findings.',
    icon: 'flask-conical',
    category: 'universal',
    capabilities: [],
    definition: {
      nodes: [
        { id: 'investigate', type: 'workflow', workflowId: 'scientific-method', acpClient: '{{primaryAgent}}', prompt: '{{inputs.topic}}' },
        { id: 'review', type: 'workflow', workflowId: 'peer-review', acpClient: '{{criticAgent}}', prompt: 'Review the investigation results for soundness.' },
      ],
      edges: [
        { source: 'investigate', target: 'review', type: 'success' },
      ],
    },
    runInputs: [
      {
        key: 'topic',
        label: 'Topic or question',
        placeholder: 'e.g. Investigate the tradeoffs of X vs Y',
        required: true,
      },
    ],
    nodeLabels: {
      investigate: 'Investigate Topic',
      review: 'Peer Review',
    },
  },
  {
    id: 'think-and-critique',
    label: 'Think & Critique',
    description: 'Deeply reason through a problem, then critically evaluate the reasoning.',
    icon: 'brain',
    category: 'universal',
    capabilities: [],
    definition: {
      nodes: [
        { id: 'reason', type: 'sequentialThinking', topic: '{{inputs.topic}}', agent: '{{primaryAgent}}', totalThoughts: 8 },
        { id: 'critique', type: 'workflow', workflowId: 'critical-thinking', acpClient: '{{criticAgent}}', prompt: 'Critically evaluate the reasoning above.' },
      ],
      edges: [
        { source: 'reason', target: 'critique', type: 'success' },
      ],
    },
    runInputs: [
      {
        key: 'topic',
        label: 'Topic or question',
        placeholder: 'e.g. Think through an architecture decision',
        required: true,
      },
    ],
    nodeLabels: {
      reason: 'Deep Reasoning',
      critique: 'Critical Review',
    },
  },
  {
    id: 'draft-and-refine',
    label: 'Draft & Refine',
    description: 'Draft a response, stress-test it via falsification, then revise.',
    icon: 'pen-tool',
    category: 'universal',
    capabilities: [],
    definition: {
      nodes: [
        { id: 'draft', type: 'prompt', prompt: '{{inputs.topic}}', agent: '{{primaryAgent}}' },
        { id: 'stress-test', type: 'workflow', workflowId: 'falsification', acpClient: '{{criticAgent}}', prompt: 'Attempt to falsify the draft above.' },
        { id: 'revise', type: 'prompt', prompt: 'Incorporate the falsification feedback and produce a refined version.', agent: '{{primaryAgent}}' },
      ],
      edges: [
        { source: 'draft', target: 'stress-test', type: 'success' },
        { source: 'stress-test', target: 'revise', type: 'success' },
      ],
    },
    runInputs: [
      {
        key: 'topic',
        label: 'Topic or question',
        placeholder: 'e.g. Draft a response about an upcoming launch',
        required: true,
      },
    ],
    nodeLabels: {
      draft: 'Initial Draft',
      'stress-test': 'Stress Test',
      revise: 'Refined Output',
    },
  },
  {
    id: 'explore-and-synthesize',
    label: 'Explore & Synthesize',
    description: 'Run agentic research, then synthesize findings through critical analysis.',
    icon: 'globe',
    category: 'universal',
    capabilities: [],
    definition: {
      nodes: [
        { id: 'explore', type: 'workflow', workflowId: 'agentic-research', acpClient: '{{primaryAgent}}', prompt: '{{inputs.topic}}' },
        { id: 'synthesize', type: 'workflow', workflowId: 'crta', acpClient: '{{criticAgent}}', prompt: 'Synthesize and critically analyze the research findings.' },
      ],
      edges: [
        { source: 'explore', target: 'synthesize', type: 'success' },
      ],
    },
    runInputs: [
      {
        key: 'topic',
        label: 'Topic or question',
        placeholder: 'e.g. Explore a market, customer, or technical topic',
        required: true,
      },
    ],
    nodeLabels: {
      explore: 'Explore Topic',
      synthesize: 'Synthesize Findings',
    },
  },

  // ── Enterprise templates (capability-gated) ─────────────────────────
  {
    id: 'deal-preparation',
    label: 'Deal Preparation',
    description: 'Fetch account data from MSX, query WorkIQ, then analyze for deal readiness.',
    icon: 'briefcase',
    category: 'enterprise',
    capabilities: [
      { toolName: 'msx-mcp-get_account_overview' },
      { toolName: 'workiq-ask_work_iq' },
    ],
    definition: {
      nodes: [
        { id: 'fetch-account', type: 'tool', toolName: 'msx-mcp-get_account_overview', toolArgs: { account_name: '{{inputs.accountName}}' } },
        { id: 'query-workiq', type: 'tool', toolName: 'workiq-ask_work_iq', toolArgs: { question: 'What is the current engagement status for {{inputs.accountName}}?' } },
        { id: 'analyze', type: 'prompt', prompt: 'Analyze the account overview and WorkIQ data to assess deal readiness for {{inputs.accountName}}.', agent: '{{primaryAgent}}' },
      ],
      edges: [
        { source: 'fetch-account', target: 'analyze', type: 'success' },
        { source: 'query-workiq', target: 'analyze', type: 'success' },
      ],
    },
    runInputs: [
      {
        key: 'accountName',
        label: 'Account name',
        description: 'Used for the MSX and WorkIQ queries at run time.',
        placeholder: 'e.g. Contoso Ltd',
        required: true,
      },
    ],
    nodeLabels: {
      'fetch-account': 'Fetch Account Overview',
      'query-workiq': 'Query Work Intelligence',
      analyze: 'Deal Readiness Analysis',
    },
  },
] as const satisfies readonly PipelineTemplate[];

export function getPipelineTemplateById(id: string): PipelineTemplate | undefined {
  return PIPELINE_TEMPLATES.find((template) => template.id === id);
}
