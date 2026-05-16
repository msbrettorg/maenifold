// Refresh, draft, and serialization helpers extracted from pipeline.ts
// to keep the main store under the 300 LOC ceiling.
import type { Ref } from 'vue';
import type { usePipelineMcp } from '../composables/usePipelineMcp';
import {
  createNode,
  createNodeId,
} from '../lib/pipeline-node-factory';
import type {
  PipelineDefinition,
  PipelineEdge,
  PipelineInputs,
} from '../lib/pipeline-types-edge';

interface ToolDefinition {
  name: string;
  description?: string;
  inputSchema?: unknown;
}

interface AssetSummaryOption {
  id: string;
}

export function serializeEdge(edge: PipelineEdge): PipelineEdge {
  if (edge.type === 'conditional') {
    const path = edge.condition?.path?.trim() ?? '';
    if (path.length === 0) {
      throw new Error(
        `Conditional edge ${edge.source} → ${edge.target} requires a non-empty path.`,
      );
    }
    const equals = edge.condition?.equals;
    if (!['string', 'number', 'boolean'].includes(typeof equals)) {
      throw new Error(
        `Conditional edge ${edge.source} → ${edge.target} requires a string, number, or boolean equals value.`,
      );
    }
  }

  return {
    source: edge.source,
    target: edge.target,
    ...(edge.type ? { type: edge.type } : {}),
    ...(edge.type === 'conditional' && edge.condition
      ? { path: edge.condition.path.trim(), equals: edge.condition.equals }
      : {}),
    ...(edge.mapping && Object.keys(edge.mapping).length > 0
      ? { mapping: { ...edge.mapping } }
      : {}),
  };
}

export function withLocalMetadata(
  definition: PipelineDefinition,
  metadata?: { templateId?: string },
): PipelineDefinition {
  const templateId = metadata?.templateId;
  return {
    ...definition,
    ...(templateId ? { templateId } : {}),
  };
}

export function buildIntentDefinition(prompt: string, agent: string): PipelineDefinition {
  const nodeId = createNodeId('prompt');
  return {
    nodes: [
      createNode('prompt', {
        id: nodeId,
        prompt,
        agent,
        displayLabel: 'Intent',
      }),
    ],
    edges: [],
  };
}

interface RefreshActionsDeps {
  mcp: ReturnType<typeof usePipelineMcp>;
  workflows: Ref<string[]>;
  tools: Ref<ToolDefinition[]>;
  roleAssets: Ref<AssetSummaryOption[]>;
  colorAssets: Ref<AssetSummaryOption[]>;
  perspectiveAssets: Ref<AssetSummaryOption[]>;
  withLoading: <T>(fn: () => Promise<T>) => Promise<T>;
}

export function createPipelineRefreshActions(deps: RefreshActionsDeps) {
  const { mcp, workflows, tools, roleAssets, colorAssets, perspectiveAssets, withLoading } = deps;

  async function refreshWorkflows() {
    await withLoading(async () => {
      try { workflows.value = await mcp.listWorkflows(); } catch { /* non-critical */ }
    });
  }

  async function refreshTools() {
    await withLoading(async () => {
      try { tools.value = await mcp.listTools(); } catch { /* non-critical */ }
    });
  }

  async function refreshAdoptAssets() {
    await withLoading(async () => {
      try {
        const [roles, colors, perspectives] = await Promise.all([
          mcp.listAssets('role'),
          mcp.listAssets('color'),
          mcp.listAssets('perspective'),
        ]);
        roleAssets.value = roles
          .map((asset) => ({ id: asset.id }))
          .filter((asset) => asset.id.trim().length > 0);
        colorAssets.value = colors
          .map((asset) => ({ id: asset.id }))
          .filter((asset) => asset.id.trim().length > 0);
        perspectiveAssets.value = perspectives
          .map((asset) => ({ id: asset.id }))
          .filter((asset) => asset.id.trim().length > 0);
      } catch {
        // Non-critical for pipeline editing.
      }
    });
  }

  return { refreshWorkflows, refreshTools, refreshAdoptAssets };
}

export function createRunInputDraftActions(runInputDrafts: Ref<Record<string, PipelineInputs>>) {
  function getRunInputDraft(pipelineSessionId: string | null | undefined): PipelineInputs {
    if (!pipelineSessionId) return {};
    return { ...(runInputDrafts.value[pipelineSessionId] ?? {}) };
  }
  function setRunInputDraft(pipelineSessionId: string, key: string, value: string) {
    runInputDrafts.value = {
      ...runInputDrafts.value,
      [pipelineSessionId]: {
        ...(runInputDrafts.value[pipelineSessionId] ?? {}),
        [key]: value,
      },
    };
  }
  function replaceRunInputDraft(pipelineSessionId: string, inputs: PipelineInputs) {
    runInputDrafts.value = {
      ...runInputDrafts.value,
      [pipelineSessionId]: { ...inputs },
    };
  }
  return { getRunInputDraft, setRunInputDraft, replaceRunInputDraft };
}

export function seedRunInputDraftFromView(
  runInputDrafts: Ref<Record<string, PipelineInputs>>,
  view: { pipelineId?: string | null; inputs?: PipelineInputs | null } | null,
) {
  if (!view?.pipelineId) return;
  const existingDraft = runInputDrafts.value[view.pipelineId];
  if (existingDraft && Object.keys(existingDraft).length > 0) return;
  if (!view.inputs || Object.keys(view.inputs).length === 0) return;

  runInputDrafts.value = {
    ...runInputDrafts.value,
    [view.pipelineId]: { ...view.inputs },
  };
}
