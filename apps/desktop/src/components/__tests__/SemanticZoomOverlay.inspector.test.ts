// @vitest-environment jsdom
import { beforeEach, describe, expect, it } from 'vitest';
import { flushPromises } from '@vue/test-utils';
import type { PipelineView } from '../../lib/pipeline-types-view';
import { normalizeView } from '../../stores/pipeline-view-transforms';
import {
  completedNodeWithSession,
  createNode,
  findButtonByLabel,
  findDialogContent,
  findScrollContainer,
  hugeNodeOutput,
  mountOverlay,
  setCurrentNode,
  setupSemanticZoomOverlay,
} from './SemanticZoomOverlay.helpers';

describe('SemanticZoomOverlay', () => {
  beforeEach(() => {
    setupSemanticZoomOverlay();
  });

  // Visual scroll behavior is verified via CDP UAT, not vitest.
  // JSDOM does not compute layout; offsetHeight mocks are dishonest.
  // These tests assert the structural CSS contract only.
  it('puts the scroll contract on the ScrollArea flex child', async () => {
    setCurrentNode(createNode({ output: hugeNodeOutput() }));
    const wrapper = mountOverlay();
    await flushPromises();

    const scrollContainer = findScrollContainer();
    expect(scrollContainer.classList.contains('flex-1')).toBe(true);
    expect(scrollContainer.classList.contains('min-h-0')).toBe(true);
    expect(scrollContainer.classList.contains('overflow-y-auto')).toBe(true);

    wrapper.unmount();
  });

  it('keeps footer action buttons outside the scroll container', async () => {
    setCurrentNode(createNode({
      output: hugeNodeOutput(),
      provenance: {
        agent: 'swe',
        inspectable: true,
        detail: {
          sessionId: 'acp-session-1',
        },
      },
    }));
    const completedWrapper = mountOverlay();
    await flushPromises();

    const dialogContent = findDialogContent();
    const scrollContainer = findScrollContainer();
    const reconnectButton = findButtonByLabel('Reconnect');
    const footer = reconnectButton.closest('footer');

    expect(scrollContainer.parentElement).toBe(dialogContent);
    expect(footer, 'footer controls should be present').not.toBeNull();
    expect(footer?.parentElement).toBe(dialogContent);
    expect(reconnectButton.closest('[data-slot="scroll-area"]')).toBeNull();
    expect(Array.from(dialogContent.children)).toContain(scrollContainer);
    expect(Array.from(dialogContent.children)).toContain(footer);

    completedWrapper.unmount();
  });

  it.each([
    ['workflow', completedNodeWithSession('workflow', 'acp-workflow-1')],
    ['prompt', completedNodeWithSession('prompt', 'acp-prompt-1')],
    ['sequentialThinking', completedNodeWithSession('sequentialThinking', 'acp-thinking-1')],
  ])('shows reconnect for completed %s nodes when an ACP session id exists', async (_label, node) => {
    setCurrentNode(node);
    const wrapper = mountOverlay();
    await flushPromises();

    expect(document.body.textContent).toContain('Reconnect');
    expect(wrapper.html()).not.toContain('Connect to live session');

    wrapper.unmount();
  });

  it('hides reconnect for completed tool nodes when an ACP session id exists', async () => {
    setCurrentNode(completedNodeWithSession('tool', 'acp-tool-1'));
    const wrapper = mountOverlay();
    await flushPromises();

    expect(document.body.textContent).not.toContain('Reconnect');
    expect(wrapper.html()).not.toContain('Connect to live session');

    wrapper.unmount();
  });

  it.each([
    ['undefined', undefined, false],
    ['null', null, false],
    ['empty', '', false],
    ['whitespace', '   ', false],
    ['valid', 'acp-valid-1', true],
  ])('treats %s session ids as reconnect visibility=%s', async (_label, sessionId, expectedVisible) => {
    setCurrentNode(completedNodeWithSession('workflow', sessionId));
    const wrapper = mountOverlay();
    await flushPromises();

    if (expectedVisible) {
      expect(document.body.textContent).toContain('Reconnect');
      expect(wrapper.html()).not.toContain('Connect to live session');
    } else {
      expect(document.body.textContent).not.toContain('Reconnect');
    }

    wrapper.unmount();
  });

  // Regression guard for T-244.83: the legacy `canReconnect` field is removed
  // from view payloads end-to-end. This assertion is parallel to the backend
  // negative assertions in crates/maenifold-core/src/workflow/pipeline/view_nodes_tests.rs
  // (assert!(!detail_object(...).contains_key("canReconnect"))).
  // Do not delete unless the field is being intentionally reintroduced.
  it('normalizes backend view payloads without a canReconnect field', () => {
    const backendFixture = {
      pipelineId: 'pipe-reconnect-flag-removal',
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
          sessionId: 'local-node-1',
          status: 'completed',
          attempts: 1,
          workflowId: 'workflow.alpha',
          provenance: {
            agent: 'architect',
            inspectable: true,
            detail: {
              sessionId: 'acp-remote-1',
            },
          },
        },
      ],
    } as unknown as PipelineView;

    const normalized = normalizeView(backendFixture);
    expect(normalized.nodes[0]?.provenance?.detail).toBeDefined();
    expect('canReconnect' in normalized.nodes[0]!.provenance!.detail).toBe(false);
  });
});
