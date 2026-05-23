// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { nextTick } from 'vue';
import MarkdownView from '../MarkdownView.vue';

const initializeMock = vi.hoisted(() => vi.fn());
const renderMock = vi.hoisted(() => vi.fn());
const memoryApi = vi.hoisted(() => ({
  readMemory: vi.fn(),
  writeMemory: vi.fn(),
  listMemories: vi.fn(),
}));

vi.mock('mermaid', () => ({
  default: {
    initialize: initializeMock,
    render: renderMock,
  },
}));

vi.mock('../../composables/useMemoryApi', () => ({ useMemoryApi: () => memoryApi }));

function createDeferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((fulfill) => { resolve = fulfill; });
  return { promise, resolve };
}

function primeMermaidRender() {
  const started = createDeferred<void>();
  const rendered = createDeferred<{ svg: string }>();
  renderMock.mockImplementationOnce(async () => {
    started.resolve();
    await new Promise((resolve) => setTimeout(resolve, 0));
    return rendered.promise;
  });
  return { started: started.promise, resolve: rendered.resolve };
}

afterEach(() => {
  renderMock.mockReset();
  initializeMock.mockReset();
  memoryApi.readMemory.mockReset();
  memoryApi.writeMemory.mockReset();
  memoryApi.listMemories.mockReset();
  document.body.innerHTML = '';
  vi.restoreAllMocks();
});

describe('MarkdownView teardown guards', () => {
  it('does not mutate detached mermaid blocks when async rendering resumes after unmount', async () => {
    setActivePinia(createPinia());
    const renderControl = primeMermaidRender();
    const wrapper = mount(MarkdownView, {
      props: {
        content: ['```mermaid', 'flowchart TD', '  A[Markdown] --> B[Done]', '```'].join('\n'),
      },
      attachTo: document.body,
    });

    await nextTick();
    await renderControl.started;

    const detachedPre = wrapper.find('pre').element as HTMLPreElement;
    const replaceWithSpy = vi.spyOn(detachedPre, 'replaceWith');

    wrapper.unmount();
    await nextTick();
    expect(detachedPre.isConnected).toBe(false);

    renderControl.resolve({ svg: '<svg xmlns="http://www.w3.org/2000/svg"><text>Rendered</text></svg>' });
    await flushPromises();

    expect(replaceWithSpy).not.toHaveBeenCalled();
  });
});
