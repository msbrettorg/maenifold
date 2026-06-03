/* @vitest-environment jsdom */

import { flushPromises, mount } from '@vue/test-utils';
import { nextTick } from 'vue';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const { callToolMock, initializeMock } = vi.hoisted(() => ({
  callToolMock: vi.fn(),
  initializeMock: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('../lib/mcp/client', async (importActual) => {
  const actual = await importActual<typeof import('../lib/mcp/client')>();

  return {
    ...actual,
    mcpHttpClient: {
      callTool: callToolMock,
      initialize: initializeMock,
    },
  };
});

import OnboardingWizard, { dismiss, needsOnboarding, reopen } from './OnboardingWizard.vue';

const ACTIVE_MODEL = {
  id: 'all-MiniLM-L6-v2',
  name: 'all-MiniLM-L6-v2',
  dimensions: 384,
  maxTokens: 512,
  sizeBytes: 23_000_000,
  license: 'Apache-2.0',
  installed: true,
  isActive: true,
};

const READY_CONFIG = {
  agents: [{ name: 'planner', command: 'node', args: ['planner.js'] }],
  settings: { defaultAgent: 'planner' },
};

const READY_EMBEDDING_STATUS = {
  activeModelStatus: {
    state: 'available',
    message: 'Ready',
    modelId: ACTIVE_MODEL.id,
  },
};

type GatewayExecuteArgs = {
  domain: string;
  command: string;
  parameters: {
    action?: string;
    [key: string]: unknown;
  };
};

const mountedWrappers: Array<ReturnType<typeof mount>> = [];

function jsonToolResult(payload: unknown) {
  return {
    content: [{ type: 'text', text: JSON.stringify(payload) }],
  };
}

function installGatewayMock(): void {
  callToolMock.mockImplementation(async (name: string, args: GatewayExecuteArgs) => {
    expect(name).toBe('GatewayExecute');

    const key = `${args.command}:${args.parameters.action ?? ''}`;

    switch (key) {
      case 'EmbeddingModels:list':
        return jsonToolResult({
          models: [ACTIVE_MODEL],
          currentDimensions: ACTIVE_MODEL.dimensions,
          activeModelId: ACTIVE_MODEL.id,
        });
      case 'EmbeddingModels:status':
        return jsonToolResult({
          activeModelStatus: READY_EMBEDDING_STATUS.activeModelStatus,
          loaded: true,
          activeModelId: ACTIVE_MODEL.id,
          model: {
            filename: `${ACTIVE_MODEL.id}.onnx`,
            dimensions: ACTIVE_MODEL.dimensions,
            maxTokens: ACTIVE_MODEL.maxTokens,
          },
        });
      case 'GetConfig:':
        return jsonToolResult({
          agents: READY_CONFIG.agents,
          settings: READY_CONFIG.settings,
        });
      default:
        throw new Error(`Unexpected GatewayExecute call: ${key}`);
    }
  });
}

function stubHealthyFetch() {
  const fetchMock = vi.fn().mockResolvedValue({
    ok: true,
    status: 200,
    text: vi.fn().mockResolvedValue(JSON.stringify({ status: 'ok' })),
  } as unknown as Response);

  vi.stubGlobal('fetch', fetchMock as typeof fetch);
  return fetchMock;
}

function mountWizard() {
  const wrapper = mount(OnboardingWizard, {
    attachTo: document.body,
    props: {
      open: true,
      config: {
        agents: READY_CONFIG.agents.map((agent) => ({ ...agent })),
        settings: { ...READY_CONFIG.settings },
      },
      embeddingStatus: {
        activeModelStatus: { ...READY_EMBEDDING_STATUS.activeModelStatus },
      },
    },
  });

  mountedWrappers.push(wrapper);
  return wrapper;
}

function activeStepLabel(wrapper: ReturnType<typeof mount>) {
  return wrapper.get('.wizard-step.active .wizard-step-label').text();
}

async function settle() {
  await nextTick();
  await flushPromises();
  await nextTick();
}

async function clickPrimary(wrapper: ReturnType<typeof mount>) {
  await wrapper.get('button.wizard-button.primary').trigger('click');
  await settle();
}

beforeEach(() => {
  window.localStorage.clear();
  callToolMock.mockReset();
  callToolMock.mockImplementation(async () => {
    throw new Error('Unexpected mcpHttpClient.callTool invocation');
  });
  initializeMock.mockReset();
  initializeMock.mockResolvedValue(undefined);

  vi.stubGlobal(
    'fetch',
    vi.fn(async () => {
      throw new Error('Unexpected fetch call');
    }) as typeof fetch
  );
});

afterEach(() => {
  while (mountedWrappers.length > 0) {
    mountedWrappers.pop()?.unmount();
  }

  window.localStorage.clear();
  document.body.innerHTML = '';
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe('needsOnboarding', () => {
  it('returns false when agents, default agent, and embeddings are ready', () => {
    expect(needsOnboarding(READY_CONFIG, READY_EMBEDDING_STATUS)).toBe(false);
  });

  it('returns true when no agents are configured', () => {
    expect(
      needsOnboarding(
        {
          agents: [],
          settings: { defaultAgent: 'planner' },
        },
        READY_EMBEDDING_STATUS
      )
    ).toBe(true);
  });

  it('returns true when embedding status is missing', () => {
    expect(needsOnboarding(READY_CONFIG, undefined)).toBe(true);
  });

  it('returns true when embeddings are not ready', () => {
    expect(
      needsOnboarding(READY_CONFIG, {
        activeModelStatus: {
          state: 'loading',
          modelId: ACTIVE_MODEL.id,
        },
      })
    ).toBe(true);
  });
});

describe('OnboardingWizard', () => {
  it('cycles through the five onboarding steps with the primary action', async () => {
    installGatewayMock();
    const fetchMock = stubHealthyFetch();
    const wrapper = mountWizard();

    const visitedLabels = [activeStepLabel(wrapper)];

    await clickPrimary(wrapper);
    visitedLabels.push(activeStepLabel(wrapper));
    expect(fetchMock).toHaveBeenCalledTimes(1);

    await clickPrimary(wrapper);
    visitedLabels.push(activeStepLabel(wrapper));

    await clickPrimary(wrapper);
    visitedLabels.push(activeStepLabel(wrapper));

    await clickPrimary(wrapper);
    visitedLabels.push(activeStepLabel(wrapper));

    expect(visitedLabels).toEqual([
      'Welcome',
      'Daemon health',
      'Embedding model',
      'Default agent',
      'Done',
    ]);
    expect(new Set(visitedLabels).size).toBe(5);
  });

  it('hides on dismiss and shows again on reopen', async () => {
    const wrapper = mountWizard();

    await settle();
    expect(wrapper.find('[role="dialog"]').exists()).toBe(true);

    dismiss();
    await settle();
    expect(wrapper.find('[role="dialog"]').exists()).toBe(false);

    reopen();
    await settle();
    expect(wrapper.find('[role="dialog"]').exists()).toBe(true);
  });

  it('stays hidden for a configured user but appears when Re-run setup calls reopen()', async () => {
    // Mirrors how App.vue mounts the wizard for a fully-configured user: the
    // initial :open is false (needsOnboarding === false), so the wizard does
    // not auto-show. The Settings "Re-run setup" button calls the module-level
    // reopen(), which must bring the mounted wizard back.
    const wrapper = mount(OnboardingWizard, {
      attachTo: document.body,
      props: {
        open: false,
        config: {
          agents: READY_CONFIG.agents.map((agent) => ({ ...agent })),
          settings: { ...READY_CONFIG.settings },
        },
        embeddingStatus: {
          activeModelStatus: { ...READY_EMBEDDING_STATUS.activeModelStatus },
        },
      },
    });
    mountedWrappers.push(wrapper);

    await settle();
    expect(wrapper.find('[role="dialog"]').exists()).toBe(false);

    reopen();
    await settle();
    expect(wrapper.find('[role="dialog"]').exists()).toBe(true);
  });

  it('keeps focus inside the dialog when Tab is pressed', async () => {
    const wrapper = mountWizard();

    await settle();

    const dialog = wrapper.get('[role="dialog"]');
    expect(dialog.attributes('tabindex')).toBe('-1');

    const outsideButton = document.createElement('button');
    document.body.appendChild(outsideButton);
    outsideButton.focus();
    expect(document.activeElement).toBe(outsideButton);

    const tabEvent = new KeyboardEvent('keydown', {
      key: 'Tab',
      bubbles: true,
      cancelable: true,
    });

    dialog.element.dispatchEvent(tabEvent);

    expect(tabEvent.defaultPrevented).toBe(true);
    expect(document.activeElement).not.toBe(outsideButton);
    expect(dialog.element.contains(document.activeElement)).toBe(true);

    outsideButton.remove();
  });

  it('routes gateway execution through the public callTool wrapper', async () => {
    installGatewayMock();
    const fetchMock = vi.fn();
    vi.stubGlobal('fetch', fetchMock as typeof fetch);
    const wrapper = mountWizard();

    await settle();
    await (wrapper.vm as { refreshEmbeddingData: () => Promise<void> }).refreshEmbeddingData();
    await settle();

    expect(callToolMock).toHaveBeenCalled();
    expect(callToolMock).toHaveBeenCalledWith(
      'GatewayExecute',
      expect.objectContaining({
        domain: 'maenifold.system',
        command: 'EmbeddingModels',
      })
    );
    expect(fetchMock).not.toHaveBeenCalled();
  });
});
