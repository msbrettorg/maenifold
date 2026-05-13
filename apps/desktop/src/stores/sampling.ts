import { computed, ref } from 'vue';
import { defineStore } from 'pinia';

export type SamplingAction = 'allow_once' | 'allow_always' | 'deny';

export interface SamplingDecision {
  action: SamplingAction;
  selectedModel?: string;
  selectedAgent?: string;
}

export interface SamplingRequest {
  id: string;
  serverName: string;
  modelHint?: string;
  messages: Array<{ role: string; text: string }>;
  systemPrompt?: string;
  maxTokens: number;
  queuedAt: number;
}

type SamplingResolvePayload = {
  kind: 'sampling';
  id: string;
  response: SamplingDecision;
};

export const useSamplingStore = defineStore('sampling', () => {
  const queue = ref<SamplingRequest[]>([]);
  const current = computed(() => queue.value[0] ?? null);
  const length = computed(() => queue.value.length);

  function enqueue(request: SamplingRequest): void {
    queue.value.push(request);
  }

  async function resolve(id: string, decision: SamplingDecision): Promise<void> {
    queue.value = queue.value.filter((request) => request.id !== id);
    const api = window.api as typeof window.api & {
      resolveUserPrompt: (payload: SamplingResolvePayload) => Promise<{ ok: true }>;
    };
    await api.resolveUserPrompt({
      kind: 'sampling',
      id,
      response: decision,
    });
  }

  function dismissExpired(id: string): void {
    queue.value = queue.value.filter((request) => request.id !== id);
  }

  return { queue, current, length, enqueue, resolve, dismissExpired };
});
