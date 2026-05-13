// @vitest-environment jsdom
import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { computed, defineComponent, nextTick, onMounted, onUnmounted, ref } from 'vue';
import type { SamplingRequest } from '../../stores/sampling';

type SamplingPreference = { alwaysAllow: boolean };

let samplingModalInstanceId = 0;
let preferences: Record<string, SamplingPreference> = {};

const SamplingApprovalModalProbe = defineComponent({
  name: 'SamplingApprovalModal',
  props: {
    request: { type: Object as () => SamplingRequest, required: true },
  },
  emits: ['resolve'],
  setup(props, { emit }) {
    const instanceId = ++samplingModalInstanceId;
    const countingDown = ref(preferences[props.request.serverName]?.alwaysAllow === true);
    let timer: ReturnType<typeof setTimeout> | null = null;

    onMounted(() => {
      if (countingDown.value) {
        timer = setTimeout(() => emit('resolve', { action: 'allow_once' }), 1000);
      }
    });

    onUnmounted(() => {
      if (timer) clearTimeout(timer);
    });

    const mode = computed(() => (countingDown.value ? 'countdown' : 'manual'));
    return { instanceId, mode };
  },
  template: `
    <section data-testid="sampling-modal" :data-instance-id="instanceId">
      <p v-if="mode === 'countdown'">Countdown for {{ request.serverName }}</p>
      <div v-else>
        <p>Manual mode for {{ request.serverName }}</p>
        <button type="button">Allow Once</button>
        <button type="button">Allow Always</button>
        <button type="button">Deny</button>
      </div>
    </section>
  `,
});

function request(id: string, serverName: string): SamplingRequest {
  return {
    id,
    serverName,
    modelHint: 'gpt-4.1',
    messages: [{ role: 'user', text: `${serverName} sample` }],
    systemPrompt: 'Be concise.',
    maxTokens: 512,
    queuedAt: 1_700_000_000_000,
  };
}

async function setupHost() {
  vi.resetModules();
  setActivePinia(createPinia());
  const resolveUserPrompt = vi.fn().mockResolvedValue({ ok: true });
  Object.defineProperty(window, 'api', {
    value: {
      quit: vi.fn(),
      onUserPromptRequest: vi.fn(),
      resolveUserPrompt,
    },
    configurable: true,
  });
  const [{ default: UserPromptHost }, { useSamplingStore }] = await Promise.all([
    import('../UserPromptHost.vue'),
    import('../../stores/sampling'),
  ]);
  const samplingStore = useSamplingStore();
  const wrapper = mount(UserPromptHost, {
    global: {
      stubs: {
        ElicitationModal: true,
        SamplingApprovalModal: SamplingApprovalModalProbe,
      },
    },
  });
  return { resolveUserPrompt, samplingStore, wrapper };
}

async function replaceCurrent(
  samplingStore: { queue: SamplingRequest[] },
  nextRequest: SamplingRequest,
) {
  samplingStore.queue.splice(0, 1, nextRequest);
  await nextTick();
}

describe('UserPromptHost sampling modal request.id keying', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    samplingModalInstanceId = 0;
    preferences = {};
  });

  afterEach(() => {
    vi.clearAllTimers();
    vi.useRealTimers();
    Reflect.deleteProperty(window, 'api');
  });

  it('modal_keyed_by_request_id_remounts_on_id_change', async () => {
    preferences = { foo: { alwaysAllow: true } };
    const { samplingStore, wrapper } = await setupHost();
    samplingStore.enqueue(request('req-A', 'foo'));
    await nextTick();
    const firstInstance = wrapper.get('[data-testid="sampling-modal"]').attributes('data-instance-id');

    await vi.advanceTimersByTimeAsync(500);
    await replaceCurrent(samplingStore, request('req-B', 'foo'));
    const secondInstance = wrapper.get('[data-testid="sampling-modal"]').attributes('data-instance-id');

    expect(secondInstance).not.toBe(firstInstance);
  });

  it('two_queued_requests_with_different_serverNames_do_not_cross_contaminate', async () => {
    preferences = { serverA: { alwaysAllow: true }, serverB: { alwaysAllow: false } };
    const { resolveUserPrompt, samplingStore, wrapper } = await setupHost();
    samplingStore.enqueue(request('req-A', 'serverA'));
    await nextTick();

    expect(wrapper.text()).toContain('Countdown for serverA');
    await vi.advanceTimersByTimeAsync(500);
    await replaceCurrent(samplingStore, request('req-B', 'serverB'));

    expect(wrapper.text()).not.toContain('Countdown');
    expect(wrapper.text()).toContain('Manual mode for serverB');
    await vi.advanceTimersByTimeAsync(1000);
    expect(resolveUserPrompt).not.toHaveBeenCalled();
  });
});
