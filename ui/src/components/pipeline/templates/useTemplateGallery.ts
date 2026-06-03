// Template gallery controller — wires the pure template runtime to the live
// stores (config agents, MCP tool discovery, pipelines) and owns the
// create-a-real-pipeline flow.
//
// Flow (findings #1/#2/#5):
//   resolveAgents(config) → instantiateTemplate / buildIntentDefinition
//   → validateDefinition (BLOCK on issues) → createPipeline → parse pipelineId
//   → viewPipeline(id) + refreshList().
// Create is blocked up front when a template needs an agent but none is
// configured, because validateDefinition would otherwise reject empty agents.

import { computed, ref } from 'vue';
import { storeToRefs } from 'pinia';
import { useConfigStore } from '../../../stores/config';
import { useMcpToolDiscoveryStore } from '../../../stores/mcpToolDiscovery';
import { usePipelinesStore } from '../../../stores/pipelines';
import { validateDefinition } from '../../../lib/pipelineDesignOps';
import type { PipelineDefinition } from '../../../lib/pipeline-types';
import {
  PIPELINE_TEMPLATES,
  templateRequiresAgent,
  type PipelineTemplate,
  type TemplateInputValues,
} from '../../../lib/templates/pipelineTemplates';
import {
  buildIntentDefinition,
  getAvailableTemplates,
  instantiateTemplate,
  intentPipelineName,
  missingRequiredInputs,
  resolveAgents,
  slugifyPipelineName,
  templateAvailability,
} from '../../../lib/templates/templateRuntime';

const AGENT_REQUIRED_MESSAGE =
  'No agent is configured. Add an agent in settings before creating this pipeline.';

/** Parse the `pipelineId` from a create response (best-effort, never throws). */
function parsePipelineId(result: unknown): string | null {
  if (result && typeof result === 'object' && 'pipelineId' in result) {
    const id = (result as { pipelineId?: unknown }).pipelineId;
    if (typeof id === 'string' && id.length > 0) return id;
  }
  return null;
}

export function useTemplateGallery() {
  const configStore = useConfigStore();
  const discoveryStore = useMcpToolDiscoveryStore();
  const pipelinesStore = usePipelinesStore();

  const { agentNames, defaultAgentName, hasAgents } = storeToRefs(configStore);
  const { status: discoveryStatus, toolNames } = storeToRefs(discoveryStore);

  const creating = ref(false);
  const createError = ref<string | null>(null);
  const lastCreatedId = ref<string | null>(null);

  // Templates the user may pick: universal always; enterprise only once tool
  // discovery is ready AND all its gated tools are present (fail-closed).
  const availableTemplates = computed(() =>
    getAvailableTemplates(PIPELINE_TEMPLATES, toolNames.value, discoveryStatus.value)
  );

  const resolved = computed(() => resolveAgents(agentNames.value, defaultAgentName.value));

  function availabilityOf(template: PipelineTemplate) {
    return templateAvailability(template, toolNames.value, discoveryStatus.value);
  }

  /** Kick off MCP tool discovery so enterprise gates can resolve. */
  async function ensureDiscovery(): Promise<void> {
    await discoveryStore.discover();
  }

  // Shared post-create step: parse the id, open the created pipeline, refresh
  // the list so it appears immediately.
  async function openCreated(result: unknown): Promise<boolean> {
    const pipelineId = parsePipelineId(result);
    if (!pipelineId) {
      createError.value = 'Pipeline was created but no id was returned.';
      return false;
    }
    lastCreatedId.value = pipelineId;
    await pipelinesStore.viewPipeline(pipelineId);
    await pipelinesStore.refreshList();
    return true;
  }

  // Validate first, then create. Returns true on success.
  async function createValidated(
    definition: PipelineDefinition,
    options: { name?: string; description?: string }
  ): Promise<boolean> {
    const issues = validateDefinition(definition);
    if (issues.length > 0) {
      createError.value = issues.map((issue) => `${issue.ref}: ${issue.message}`).join('; ');
      return false;
    }
    const result = await pipelinesStore.createPipeline(definition, options);
    return openCreated(result);
  }

  /** Instantiate a template with the resolved agents + inputs and create it. */
  async function createFromTemplate(
    template: PipelineTemplate,
    inputValues: TemplateInputValues
  ): Promise<boolean> {
    createError.value = null;

    if (availabilityOf(template) !== 'available') {
      createError.value = 'This template is not available yet.';
      return false;
    }

    const missing = missingRequiredInputs(template, inputValues);
    if (missing.length > 0) {
      createError.value = `Missing required input(s): ${missing.join(', ')}.`;
      return false;
    }

    const agents = resolved.value;
    if (templateRequiresAgent(template) && !agents.primaryAgent) {
      createError.value = AGENT_REQUIRED_MESSAGE;
      return false;
    }

    creating.value = true;
    try {
      const { definition } = instantiateTemplate(template, {
        primaryAgent: agents.primaryAgent,
        criticAgent: agents.criticAgent,
        inputs: inputValues,
      });
      return await createValidated(definition, {
        name: slugifyPipelineName(template.label, template.id),
        description: template.description,
      });
    } catch (error) {
      createError.value = error instanceof Error ? error.message : String(error);
      return false;
    } finally {
      creating.value = false;
    }
  }

  /** Build a one-node pipeline from a free-form intent and create it. */
  async function createFromIntent(intent: string): Promise<boolean> {
    createError.value = null;
    const trimmed = intent.trim();
    if (!trimmed) {
      createError.value = 'Describe what you want the pipeline to do.';
      return false;
    }

    const agents = resolved.value;
    if (!agents.primaryAgent) {
      createError.value = AGENT_REQUIRED_MESSAGE;
      return false;
    }

    creating.value = true;
    try {
      const definition = buildIntentDefinition(trimmed, agents.primaryAgent);
      return await createValidated(definition, {
        name: slugifyPipelineName(intentPipelineName(trimmed), 'pipeline'),
      });
    } catch (error) {
      createError.value = error instanceof Error ? error.message : String(error);
      return false;
    } finally {
      creating.value = false;
    }
  }

  return {
    // state
    creating,
    createError,
    lastCreatedId,
    discoveryStatus,
    hasAgents,
    resolved,
    availableTemplates,
    // actions
    availabilityOf,
    ensureDiscovery,
    createFromTemplate,
    createFromIntent,
  };
}
