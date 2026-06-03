/* @vitest-environment jsdom */

// Blue-team coverage (T-600.7, Defect 3: no fake surfaces). The lineage* values
// are stable lineage HASHES (`lin-v1-<kind>-<sha256-prefix>`, see
// crates/maenifold-core/src/lineage.rs), NOT pipeline session ids or memory uris.
// They have no in-app destination, so every breadcrumb must render as INERT
// context — never a clickable button/link that would dead-end on a lookup miss.

import { describe, expect, it } from 'vitest';
import { mount } from '@vue/test-utils';

import ProvenanceBreadcrumbs from './ProvenanceBreadcrumbs.vue';
import type { PromptViewNode, ViewNode } from '../../../lib/pipeline-types';

function nodeWithLineage(): ViewNode {
  const node: PromptViewNode = {
    type: 'prompt',
    nodeId: 'n-prompt',
    sessionId: 'pipeline-sess',
    status: 'active',
    attempts: 1,
    prompt: 'go',
    agent: 'copilot',
    provenance: {
      agent: 'copilot',
      inspectable: true,
      detail: {
        lineageGateTaskId: 'lin-v1-gate-task-aaaaaaaaaaaaaaaaaaaa',
        lineagePipelineRunId: 'lin-v1-pipeline-run-bbbbbbbbbbbbbbbbbbbb',
        lineageThinkingSessionId: 'lin-v1-thinking-session-cccccccccccccccccccc',
        lineageMemoryNoteId: 'lin-v1-memory-note-dddddddddddddddddddd',
      },
    },
  };
  return node;
}

describe('ProvenanceBreadcrumbs', () => {
  it('renders every lineage crumb as inert (no clickable button/link)', () => {
    const wrapper = mount(ProvenanceBreadcrumbs, { props: { node: nodeWithLineage() } });

    // All four lineage kinds present...
    const crumbs = wrapper.findAll('.prov-crumb');
    expect(crumbs).toHaveLength(4);

    // ...but NONE of them are interactive: no buttons, no link styling, no
    // navigable destination fabricated for an unresolvable lin-v1 hash.
    expect(wrapper.findAll('button')).toHaveLength(0);
    expect(wrapper.findAll('.prov-crumb--link')).toHaveLength(0);
    for (const crumb of crumbs) {
      expect(crumb.element.tagName).toBe('SPAN');
    }

    // The pipeline-run / memory-note lineage hashes are shown verbatim as text.
    expect(wrapper.text()).toContain('lin-v1-pipeline-run-bbbbbbbbbbbbbbbbbbbb');
    expect(wrapper.text()).toContain('lin-v1-memory-note-dddddddddddddddddddd');
  });

  it('renders nothing when the node carries no lineage', () => {
    const node: PromptViewNode = {
      type: 'prompt',
      nodeId: 'n2',
      sessionId: 's',
      status: 'active',
      attempts: 1,
      prompt: 'go',
      agent: 'copilot',
    };
    const wrapper = mount(ProvenanceBreadcrumbs, { props: { node } });
    expect(wrapper.find('.prov').exists()).toBe(false);
  });
});
