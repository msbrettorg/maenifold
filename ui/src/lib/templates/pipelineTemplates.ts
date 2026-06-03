// Pipeline template registry — pure typed data.
//
// Ported from the old Electron UI's pipeline-templates.ts (catalog data only)
// and re-typed onto the new `pipeline-types.ts` shapes. Each template declares an
// intent-first label, a description, a raw `PipelineDefinition` whose string
// leaves carry `{{variable}}` placeholders, a `nodeLabels` authoring map that the
// runtime stamps onto instantiated nodes as `displayLabel`, the run-time inputs
// collected before instantiation, and optional capability gates that hide the
// template until the required MCP tools are discovered.
//
// Templates are resolved at instantiation time by `templateRuntime.ts`.

import type { PipelineDefinition } from '../pipeline-types';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type TemplateInputValues = Record<string, string>;

/** Structural + user-provided variables interpolated before creating a pipeline. */
export interface TemplateVariables {
  primaryAgent: string;
  criticAgent: string;
  inputs?: TemplateInputValues;
}

/** An MCP tool that must be discovered for the template to be available. */
export interface TemplateCapabilityGate {
  /** Fully-qualified MCP tool name, e.g. "msx-mcp-get_account_overview". */
  toolName: string;
}

export type TemplateCategory = 'universal' | 'enterprise';
export type TemplateInputType = 'text' | 'textarea';

export interface TemplateInputDefinition {
  /** Key used by `{{inputs.key}}` placeholders. */
  key: string;
  /** Human-readable label shown in the create form. */
  label: string;
  /** Optional helper copy under the field label. */
  description?: string;
  /** Placeholder text for the create-time field. */
  placeholder?: string;
  /** Preferred editor for the create-time field. */
  type?: TemplateInputType;
  /** Whether create is blocked until a non-empty value is provided. */
  required?: boolean;
}

export interface PipelineTemplate {
  /** Unique slug used as registry key. */
  id: string;
  /** Intent-first label shown in the gallery. */
  label: string;
  /** One-line description of what this pipeline does. */
  description: string;
  /** Icon key for @lucide/vue (kebab-case maps to the PascalCase component). */
  icon: string;
  category: TemplateCategory;
  /** When non-empty, ALL listed tools must be discovered for the template to appear. */
  capabilities: readonly TemplateCapabilityGate[];
  /** Raw pipeline definition with `{{variable}}` placeholders in string values. */
  definition: PipelineDefinition;
  /** Inputs collected before instantiation to resolve `{{inputs.*}}` placeholders. */
  runInputs: readonly TemplateInputDefinition[];
  /** Map from authoring node id → human-readable label (stamped as displayLabel). */
  nodeLabels: Readonly<Record<string, string>>;
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

// Typed (not `as const`) so the annotation supplies contextual typing: the
// `type: 'success'` edge discriminators and node `type` literals are checked
// against their unions rather than being widened to `string`.
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
        { type: 'workflow', id: 'investigate', workflowId: 'scientific-method', acpClient: '{{primaryAgent}}', prompt: '{{inputs.topic}}' },
        { type: 'workflow', id: 'review', workflowId: 'peer-review', acpClient: '{{criticAgent}}', prompt: 'Review the investigation results for soundness.' },
      ],
      edges: [{ source: 'investigate', target: 'review', type: 'success' }],
    },
    runInputs: [
      {
        key: 'topic',
        label: 'Topic or question',
        placeholder: 'e.g. Investigate the tradeoffs of X vs Y',
        type: 'textarea',
        required: true,
      },
    ],
    nodeLabels: { investigate: 'Investigate Topic', review: 'Peer Review' },
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
        { type: 'sequentialThinking', id: 'reason', topic: '{{inputs.topic}}', agent: '{{primaryAgent}}', totalThoughts: 8 },
        { type: 'workflow', id: 'critique', workflowId: 'critical-thinking', acpClient: '{{criticAgent}}', prompt: 'Critically evaluate the reasoning above.' },
      ],
      edges: [{ source: 'reason', target: 'critique', type: 'success' }],
    },
    runInputs: [
      {
        key: 'topic',
        label: 'Topic or question',
        placeholder: 'e.g. Think through an architecture decision',
        type: 'textarea',
        required: true,
      },
    ],
    nodeLabels: { reason: 'Deep Reasoning', critique: 'Critical Review' },
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
        { type: 'prompt', id: 'draft', prompt: '{{inputs.topic}}', agent: '{{primaryAgent}}' },
        { type: 'workflow', id: 'stress-test', workflowId: 'falsification', acpClient: '{{criticAgent}}', prompt: 'Attempt to falsify the draft above.' },
        { type: 'prompt', id: 'revise', prompt: 'Incorporate the falsification feedback and produce a refined version.', agent: '{{primaryAgent}}' },
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
        type: 'textarea',
        required: true,
      },
    ],
    nodeLabels: { draft: 'Initial Draft', 'stress-test': 'Stress Test', revise: 'Refined Output' },
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
        { type: 'workflow', id: 'explore', workflowId: 'agentic-research', acpClient: '{{primaryAgent}}', prompt: '{{inputs.topic}}' },
        { type: 'workflow', id: 'synthesize', workflowId: 'crta', acpClient: '{{criticAgent}}', prompt: 'Synthesize and critically analyze the research findings.' },
      ],
      edges: [{ source: 'explore', target: 'synthesize', type: 'success' }],
    },
    runInputs: [
      {
        key: 'topic',
        label: 'Topic or question',
        placeholder: 'e.g. Explore a market, customer, or technical topic',
        type: 'textarea',
        required: true,
      },
    ],
    nodeLabels: { explore: 'Explore Topic', synthesize: 'Synthesize Findings' },
  },

  // ── Enterprise templates (capability-gated) ─────────────────────────
  {
    id: 'deal-preparation',
    label: 'Deal Preparation',
    description: 'Fetch account data from MSX, query WorkIQ, then analyze for deal readiness.',
    icon: 'briefcase',
    category: 'enterprise',
    capabilities: [{ toolName: 'msx-mcp-get_account_overview' }, { toolName: 'workiq-ask_work_iq' }],
    definition: {
      nodes: [
        { type: 'tool', id: 'fetch-account', toolName: 'msx-mcp-get_account_overview', toolArgs: { account_name: '{{inputs.accountName}}' } },
        { type: 'tool', id: 'query-workiq', toolName: 'workiq-ask_work_iq', toolArgs: { question: 'What is the current engagement status for {{inputs.accountName}}?' } },
        { type: 'prompt', id: 'analyze', prompt: 'Analyze the account overview and WorkIQ data to assess deal readiness for {{inputs.accountName}}.', agent: '{{primaryAgent}}' },
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
];

export function getPipelineTemplateById(id: string): PipelineTemplate | undefined {
  return PIPELINE_TEMPLATES.find((template) => template.id === id);
}

/** True when a template has prompt/think nodes (or ACP workflow nodes) that need an agent. */
export function templateRequiresAgent(template: PipelineTemplate): boolean {
  return template.definition.nodes.some((node) => {
    const type = node.type ?? 'workflow';
    if (type === 'prompt' || type === 'sequentialThinking') return true;
    if (type === 'workflow') {
      const acpClient = (node as { acpClient?: string }).acpClient;
      return typeof acpClient === 'string' && acpClient.length > 0;
    }
    return false;
  });
}
