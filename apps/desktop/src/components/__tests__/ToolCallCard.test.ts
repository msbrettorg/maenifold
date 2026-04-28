// @vitest-environment jsdom
import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { describe, expect, it } from 'vitest';
import ToolCallCard from '../ToolCallCard.vue';
import ToolCallCardExtensions from '../ToolCallCardExtensions.vue';
import type { ToolCallInfo } from '../../types/acp';
import { useAppShellStore } from '../../stores/app-shell';
import { useKnowledgeStore } from '../../stores/knowledge';

function createToolCall(overrides: Partial<ToolCallInfo> = {}): ToolCallInfo {
  return {
    toolCallId: 'tool-1',
    title: 'Read project file',
    kind: 'read',
    status: 'in_progress',
    locations: [{ path: '/repo/README.md' }],
    ...overrides,
  };
}

describe('ToolCallCard', () => {
  it('renders status indicator, kind icon, and locations from ToolCallInfo', () => {
    const wrapper = mount(ToolCallCard, {
      props: { toolCall: createToolCall() },
    });

    expect(wrapper.text()).toContain('📖');
    expect(wrapper.text()).toContain('⚙️');
    expect(wrapper.text()).toContain('Read project file');
    expect(wrapper.text()).toContain('/repo/README.md');
    expect(wrapper.find('.status-running').exists()).toBe(true);
    expect(wrapper.find('.kind-icon').exists()).toBe(true);
    expect(wrapper.find('.location').exists()).toBe(true);
  });

  it('renders Maenifold extension slot content for output, diffs, raw payload, and location chips', async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const appShellStore = useAppShellStore();
    const knowledgeStore = useKnowledgeStore();
    const toolCall = createToolCall({
      kind: 'edit',
      status: 'completed',
      locations: [{ path: 'memory://TestNote' }, { path: '/repo/src/main.ts' }],
      content: [
        { type: 'content', content: { type: 'text', text: '**done**' } },
        { type: 'diff', path: '/repo/src/main.ts', oldText: 'old', newText: 'new' },
      ],
      rawInput: { command: 'edit' },
      rawOutput: { ok: true },
    } as Partial<ToolCallInfo>);
    const slotted = mount(ToolCallCard, {
      props: { toolCall },
      slots: {
        details: '<div class="extension-slot">extension hook</div>',
      },
    });
    const wrapper = mount(ToolCallCardExtensions, {
      props: { toolCall: toolCall as never, expanded: true },
      global: {
        plugins: [pinia],
        stubs: { MarkdownView: { template: '<article>{{ content }}</article>', props: ['content'] } },
      },
    });

    expect(slotted.find('.extension-slot').exists()).toBe(true);
    expect(wrapper.text()).toContain('memory://TestNote');
    expect(wrapper.text()).toContain('/repo/src/main.ts');
    expect(wrapper.text()).toContain('**done**');
    expect(wrapper.text()).toContain('--- /repo/src/main.ts');
    expect(wrapper.text()).toContain('+++ /repo/src/main.ts');

    const absoluteLocation = wrapper.findAll('button').find((button) => button.text().includes('/repo/src/main.ts'));
    expect(absoluteLocation).toBeTruthy();
    await absoluteLocation!.trigger('click');
    expect(appShellStore.activeTab).toBe('home');
    expect(knowledgeStore.activeTabUri).toBeNull();

    const memoryLocation = wrapper.findAll('button').find((button) => button.text().includes('memory://TestNote'));
    expect(memoryLocation).toBeTruthy();
    await memoryLocation!.trigger('click');
    expect(appShellStore.activeTab).toBe('knowledge');
    expect(knowledgeStore.activeTabUri).toBe('memory://TestNote');

    const rawToggle = wrapper.findAll('button').find((button) => button.text().includes('View raw'));
    expect(rawToggle).toBeTruthy();
    await rawToggle!.trigger('click');
    expect(wrapper.text()).toContain('tool_call');
    expect(wrapper.text()).toContain('tool_call_update');
  });
});
