// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { flushPromises, mount } from '@vue/test-utils';

import PipelineInspector from '../PipelineInspector.vue';
import { normalizeAgentProbeResult } from '../../lib/agent-probe-types';
import type { PipelineView } from '../../lib/pipeline-types';
import { useConfigStore } from '../../stores/config';
import { useAgentCapabilitiesStore } from '../../stores/agent-capabilities';
import { usePipelineStore } from '../../stores/pipeline';
import { toLocalDefinition } from '../../stores/pipeline-view-transforms';

function createPromptRoundTripView(): PipelineView {
  return {
    pipelineId: 'pipeline-1',
    status: 'paused',
    running: false,
    ready: [],
    active: [],
    completed: [],
    mermaid: 'graph TD;',
    edges: [],
    nodes: [
      {
        nodeId: 'prompt-1',
        sessionId: 'session-1',
        status: 'pending',
        attempts: 0,
        type: 'prompt',
        prompt: 'Investigate the failure',
        agent: 'architect',
        model: 'stale-model',
        thoughtLevel: 'medium',
        agentConfig: {
          mode: 'prompt',
          model: 'gpt-4.1',
        },
      },
    ],
  };
}

function createToolProvenanceView(): PipelineView {
  return {
    pipelineId: 'pipeline-2',
    status: 'paused',
    running: false,
    ready: [],
    active: [],
    completed: ['tool-1'],
    mermaid: 'graph TD;',
    edges: [],
    nodes: [
      {
        nodeId: 'tool-1',
        sessionId: 'workflow-tool-local',
        status: 'completed',
        attempts: 1,
        type: 'tool',
        toolName: 'example.echo',
        toolProvenance: {
          toolName: 'example.echo',
          resolvedArgs: { value: 1 },
          responseSummary: '{"value":1}',
        },
      },
    ],
  };
}

function createWorkflowClientResetView(): PipelineView {
  return {
    pipelineId: 'pipeline-3',
    status: 'paused',
    running: false,
    ready: [],
    active: [],
    completed: [],
    mermaid: 'graph TD;',
    edges: [],
    nodes: [
      {
        nodeId: 'workflow-1',
        sessionId: 'workflow-session-1',
        status: 'pending',
        attempts: 0,
        type: 'workflow',
        workflowId: 'workflow.alpha',
        agent: 'architect',
        prompt: 'Run the workflow',
        agentConfig: {
          mode: 'prompt',
          model: 'gpt-4.1',
          thoughtLevel: 'medium',
        },
      },
    ],
  };
}

function createTestEnv() {
  const pinia = createPinia();
  setActivePinia(pinia);

  const configStore = useConfigStore();
  configStore.config = {
    mcpServers: [],
    agents: {
      architect: { command: 'architect' },
      reviewer: { command: 'reviewer' },
    },
  };

  const capabilitiesStore = useAgentCapabilitiesStore();
  capabilitiesStore.probeResults = new Map([
    ['architect', normalizeAgentProbeResult({
      configOptions: [
        {
          id: 'mode',
          name: 'Mode',
          category: 'mode',
          currentValue: 'prompt',
          options: [
            { value: 'prompt', name: 'Prompt' },
            { value: 'workflow', name: 'Workflow' },
          ],
        },
        {
          id: 'model',
          name: 'Model',
          category: 'model',
          currentValue: 'gpt-4.1',
          options: [
            { value: 'gpt-4.1', name: 'GPT-4.1' },
            { value: 'gpt-5-mini', name: 'GPT-5 mini' },
          ],
        },
        {
          id: 'thought_level',
          name: 'Thought Level',
          category: 'thought_level',
          currentValue: 'medium',
          options: [
            { value: 'low', name: 'Low' },
            { value: 'medium', name: 'Medium' },
            { value: 'high', name: 'High' },
          ],
        },
      ],
    })],
    ['reviewer', normalizeAgentProbeResult({
      configOptions: [
        {
          id: 'mode',
          name: 'Mode',
          category: 'mode',
          currentValue: 'workflow',
          options: [
            { value: 'prompt', name: 'Prompt' },
            { value: 'workflow', name: 'Workflow' },
          ],
        },
        {
          id: 'model',
          name: 'Model',
          category: 'model',
          currentValue: 'gpt-5-mini',
          options: [
            { value: 'gpt-4.1', name: 'GPT-4.1' },
            { value: 'gpt-5-mini', name: 'GPT-5 mini' },
          ],
        },
        {
          id: 'thought_level',
          name: 'Thought Level',
          category: 'thought_level',
          currentValue: 'high',
          options: [
            { value: 'low', name: 'Low' },
            { value: 'medium', name: 'Medium' },
            { value: 'high', name: 'High' },
          ],
        },
      ],
    })],
  ]);

  const pipelineStore = usePipelineStore();
  pipelineStore.designMode = true;
  pipelineStore.localDefinition = toLocalDefinition(createPromptRoundTripView());
  pipelineStore.workflows = ['workflow.alpha'];

  return { pinia, pipelineStore };
}

describe('PipelineInspector', () => {
  it('round-trips viewed prompt nodes through agentConfig-only edits', async () => {
    const { pinia, pipelineStore } = createTestEnv();

    const wrapper = mount(PipelineInspector, {
      props: {
        selectedNodeId: 'prompt-1',
        selectedEdge: null,
      },
      global: {
        plugins: [pinia],
      },
    });

    await flushPromises();

    const modeBadges = wrapper.findAll('.agent-mode-badges .mode-badge');
    expect(modeBadges.map((badge) => badge.text())).toEqual(['Prompt', 'Workflow']);

    const configFields = wrapper.findAll('.agent-config-grid .field');
    const modelField = configFields.find((field) => field.find('span').text() === 'Model');
    const thoughtLevelField = configFields.find((field) => field.find('span').text() === 'Thought Level');

    expect(modelField?.exists()).toBe(true);
    expect(thoughtLevelField?.exists()).toBe(true);

    await modelField!.get('select').setValue('gpt-5-mini');
    await thoughtLevelField!.get('select').setValue('high');
    await flushPromises();

    expect(pipelineStore.localDefinition.nodes).toHaveLength(1);
    expect(pipelineStore.localDefinition.nodes[0]).toMatchObject({
      id: 'prompt-1',
      type: 'prompt',
      prompt: 'Investigate the failure',
      agent: 'architect',
      agentConfig: {
        mode: 'prompt',
        model: 'gpt-5-mini',
        thoughtLevel: 'high',
      },
    });
    expect('model' in pipelineStore.localDefinition.nodes[0]!).toBe(false);
    expect('thoughtLevel' in pipelineStore.localDefinition.nodes[0]!).toBe(false);
  });

  it('resets workflow agentConfig when the ACP client changes', async () => {
    const { pinia, pipelineStore } = createTestEnv();
    pipelineStore.localDefinition = toLocalDefinition(createWorkflowClientResetView());

    const wrapper = mount(PipelineInspector, {
      props: {
        selectedNodeId: 'workflow-1',
        selectedEdge: null,
      },
      global: {
        plugins: [pinia],
      },
    });

    await flushPromises();

    const configFields = wrapper.findAll('.field');
    const acpClientField = configFields.find((field) => field.find('span').text() === 'ACP client');
    expect(acpClientField?.exists()).toBe(true);

    await acpClientField!.get('select').setValue('reviewer');
    await flushPromises();

    expect(pipelineStore.localDefinition.nodes).toHaveLength(1);
    expect(pipelineStore.localDefinition.nodes[0]).toMatchObject({
      id: 'workflow-1',
      type: 'workflow',
      workflowId: 'workflow.alpha',
      acpClient: 'reviewer',
      prompt: 'Run the workflow',
      agentConfig: {
        mode: 'workflow',
        model: 'gpt-5-mini',
        thoughtLevel: 'high',
      },
    });
    expect(pipelineStore.localDefinition.nodes[0]?.agentConfig?.model).not.toBe('gpt-4.1');
    expect(pipelineStore.localDefinition.nodes[0]?.agentConfig?.thoughtLevel).not.toBe('medium');
  });

  it('shows read-only tool provenance for server-side tool nodes', async () => {
    const { pinia, pipelineStore } = createTestEnv();
    pipelineStore.designMode = false;
    pipelineStore.currentView = createToolProvenanceView();

    const wrapper = mount(PipelineInspector, {
      props: {
        selectedNodeId: 'tool-1',
        selectedEdge: null,
      },
      global: {
        plugins: [pinia],
      },
    });

    await flushPromises();

    expect(wrapper.text()).toContain('Tool node — executed directly, no interactive session');
    expect(wrapper.text()).toContain('example.echo');
    expect(wrapper.text()).toContain('Response summary');
    expect(wrapper.text()).toContain('{"value":1}');
    expect(wrapper.text()).toContain('"value": 1');
  });
});
