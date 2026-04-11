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

function createTestEnv() {
  const pinia = createPinia();
  setActivePinia(pinia);

  const configStore = useConfigStore();
  configStore.config = {
    mcpServers: [],
    agents: {
      architect: { command: 'architect' },
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
});
