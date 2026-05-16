// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { flushPromises, mount } from '@vue/test-utils';
import { ref } from 'vue';
import PipelineSidebar from '../PipelineSidebar.vue';

const { readResource } = vi.hoisted(() => ({ readResource: vi.fn() }));

vi.mock('../../composables/useMcpResources', () => ({
  McpCapabilityUnsupportedError: class extends Error {},
  useMcpResources: () => ({ readResource, subscribeResource: vi.fn(), unsubscribeResource: vi.fn() }),
}));
vi.mock('../../composables/usePipelineMcp', () => ({
  usePipelineMcp: () => ({
    connected: ref(true),
    error: ref(null),
    reconnecting: ref(false),
    listPipelines: vi.fn(async () => ({ pipelines: [] })),
    callTool: vi.fn(),
    connect: vi.fn(),
    disconnect: vi.fn(),
    listTools: vi.fn(async () => []),
    diffViews: vi.fn(() => []),
    startPolling: vi.fn(),
  }),
}));

describe('PipelineSidebar error sanitization', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
  });

  it('redacts sensitive paths before surfacing pipeline resource errors', async () => {
    const raw = 'read failed /Users/brett/secret/file.txt tenantId: a1b2c3d4-e5f6-1234-9abc-def012345678';
    readResource.mockRejectedValueOnce(new Error(raw));

    const wrapper = mount(PipelineSidebar, { global: { plugins: [createPinia()] } });
    await flushPromises();

    const title = wrapper.get('[aria-label="Pipeline resource fallback active"]').attributes('title');
    expect(title).toContain('<redacted>');
    expect(title).not.toContain('/Users/brett/secret/file.txt');
    expect(title).not.toContain('a1b2c3d4-e5f6-1234-9abc-def012345678');
  });
});
