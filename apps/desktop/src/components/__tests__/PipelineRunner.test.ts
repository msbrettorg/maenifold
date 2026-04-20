// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { mount } from '@vue/test-utils';
import type { PipelineView } from '../../lib/pipeline-types';
import PipelineRunner from '../PipelineRunner.vue';
import { usePipelineStore } from '../../stores/pipeline';

function createView(): PipelineView {
  return {
    pipelineId: 'pipeline-1',
    status: 'paused',
    running: false,
    ready: [],
    active: [],
    completed: ['node-1'],
    mermaid: 'graph TD;',
    edges: [],
    nodes: [
      {
        nodeId: 'node-1',
        displayLabel: 'Compile assets',
        sessionId: 'session-1',
        status: 'completed',
        attempts: 1,
        type: 'tool',
        toolName: 'example.compile',
        output: 'live output should not be used for loaded entries',
      },
    ],
  };
}

describe('PipelineRunner loaded event formatting', () => {
  it('renders loaded completed entries with the hydrated output wording', () => {
    const pinia = createPinia();
    setActivePinia(pinia);

    const store = usePipelineStore();
    store.designMode = false;
    store.currentPipelineId = 'pipeline-1';
    store.currentView = createView();
    store.localDefinition = { nodes: [], edges: [] };
    store.eventLog = [{
      nodeId: 'node-1',
      from: 'loaded',
      to: 'completed',
      timestamp: Date.parse('2026-04-11T12:00:00Z'),
      preview: 'snapshotted output preview',
    }];

    const wrapper = mount(PipelineRunner, {
      global: { plugins: [pinia] },
    });

    expect(wrapper.text()).toContain('Compile assets loaded as completed — output: snapshotted output preview');
    expect(wrapper.text()).not.toContain('live output should not be used');
  });

  it('renders abandoned entries with preview details', () => {
    const pinia = createPinia();
    setActivePinia(pinia);

    const store = usePipelineStore();
    store.designMode = false;
    store.currentPipelineId = 'pipeline-1';
    store.currentView = {
      ...createView(),
      nodes: [{
        nodeId: 'node-1',
        displayLabel: 'Compile assets',
        sessionId: 'session-1',
        status: 'abandoned',
        attempts: 1,
        type: 'tool',
        toolName: 'example.compile',
        output: 'backend error should not replace snapshotted details',
      }],
    };
    store.localDefinition = { nodes: [], edges: [] };
    store.eventLog = [{
      nodeId: 'node-1',
      from: 'active',
      to: 'abandoned',
      timestamp: Date.parse('2026-04-11T12:00:00Z'),
      preview: 'tool execution failed: missing credentials',
    }];

    const wrapper = mount(PipelineRunner, {
      global: { plugins: [pinia] },
    });

    expect(wrapper.text()).toContain('Compile assets abandoned: tool execution failed: missing credentials');
    expect(wrapper.text()).not.toContain('backend error should not replace');
  });
});
