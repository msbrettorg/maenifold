// @vitest-environment jsdom
import { mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { defineComponent, ref } from 'vue';
import { useEmbeddingsActions } from '../useEmbeddingsActions';

const mockGatewayExecute = vi.fn();
const mockConnect = vi.fn();
const mockConnected = ref(false);

vi.mock('@/composables/useToast', () => ({
  toastSuccess: vi.fn(),
  toastWarning: vi.fn(),
  toastError: vi.fn(),
}));

vi.mock('../useMcpClient', () => ({
  useMcpClient: vi.fn(() => ({
    gatewayExecute: mockGatewayExecute,
    connect: mockConnect,
    connected: mockConnected,
  })),
}));

function mountHarness() {
  let actions: ReturnType<typeof useEmbeddingsActions> | null = null;
  const wrapper = mount(defineComponent({
    setup() {
      actions = useEmbeddingsActions();
      return () => null;
    },
  }));
  return { actions: actions!, wrapper };
}

describe('useEmbeddingsActions', () => {
  beforeEach(() => {
    mockGatewayExecute.mockReset();
    mockConnect.mockReset();
    mockConnected.value = false;
  });

  it('calls EmbeddingModels with the canonical gateway envelope during initialLoad', async () => {
    mockConnect.mockResolvedValue(undefined);
    mockGatewayExecute
      .mockResolvedValueOnce({ loaded: true, model: { filename: 'bge-m3.gguf', dimensions: 1024, maxTokens: 8192 } })
      .mockResolvedValueOnce({ models: [], currentDimensions: 1024 });

    const { actions, wrapper } = mountHarness();

    await actions.initialLoad();
    wrapper.unmount();

    expect(mockConnect).toHaveBeenCalledTimes(1);
    expect(mockGatewayExecute).toHaveBeenNthCalledWith(
      1,
      'maenifold.system',
      'EmbeddingModels',
      { action: 'status' },
    );
    expect(mockGatewayExecute).toHaveBeenNthCalledWith(
      2,
      'maenifold.system',
      'EmbeddingModels',
      { action: 'list' },
    );
  });

  it('surfaces gatewayExecute errors to the caller state during initialLoad', async () => {
    mockConnect.mockResolvedValue(undefined);
    mockGatewayExecute.mockRejectedValue(new Error('embedding models unavailable'));

    const { actions, wrapper } = mountHarness();

    await actions.initialLoad();
    wrapper.unmount();

    expect(mockGatewayExecute).toHaveBeenCalledWith(
      'maenifold.system',
      'EmbeddingModels',
      { action: 'status' },
    );
    expect(actions.error.value).toBe('embedding models unavailable');
    expect(actions.loading.value).toBe(false);
  });
});
