// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { defineComponent, h } from 'vue';
import SemanticZoomOverlayContent from '../SemanticZoomOverlayContent.vue';
import type { ToolProvenance } from '../../lib/pipeline-types';
import type { DispatchedResource } from '../../lib/resource-mime-dispatcher';

type ContentProps = InstanceType<typeof SemanticZoomOverlayContent>['$props'];

const MarkdownViewStub = defineComponent({
  name: 'MarkdownView',
  props: {
    content: { type: String, required: true },
  },
  setup(props) {
    return () => h('article', { 'data-testid': 'markdown-view' }, props.content);
  },
});

const ResourcePreviewStub = defineComponent({
  name: 'ResourcePreview',
  props: {
    dispatched: { type: Object, required: true },
  },
  setup(props) {
    return () => h('section', { 'data-testid': 'resource-preview' }, JSON.stringify(props.dispatched));
  },
});

function mountContent(overrides: Partial<ContentProps> = {}) {
  setActivePinia(createPinia());
  return mount(SemanticZoomOverlayContent, {
    props: {
      assembledPrompt: null,
      nodeOutput: null,
      thinkingTrace: null,
      toolProvenance: null,
      workflowSteps: null,
      ...overrides,
    },
    global: {
      stubs: {
        Accordion: { template: '<section data-testid="accordion"><slot /></section>' },
        AccordionItem: { props: ['value'], template: '<section :data-accordion-item="value"><slot /></section>' },
        AccordionTrigger: { template: '<button type="button"><slot /></button>' },
        AccordionContent: { template: '<div><slot /></div>' },
        MarkdownView: MarkdownViewStub,
        ResourcePreview: ResourcePreviewStub,
        ThinkingTraceView: true,
        PromptAssemblyPreview: true,
        WorkflowStepProgress: true,
      },
    },
  });
}

function resourcePreviewProps(wrapper: ReturnType<typeof mountContent>): DispatchedResource | null {
  const preview = wrapper.findComponent({ name: 'ResourcePreview' });
  return preview.exists() ? preview.props('dispatched') as DispatchedResource : null;
}

describe('SemanticZoomOverlayContent unified output rendering', () => {
  it('renders node output via MarkdownView or the unified dispatcher, not raw pre', () => {
    const output = '# title\n\n```ts\nconst ok = true;\n```';
    const wrapper = mountContent({ nodeOutput: output });

    const markdown = wrapper.findComponent({ name: 'MarkdownView' });
    const dispatched = resourcePreviewProps(wrapper);

    expect(wrapper.find('[data-accordion-item="output"] pre').exists()).toBe(false);
    expect(markdown.exists() || dispatched?.kind === 'markdown').toBe(true);
    if (markdown.exists()) {
      expect(markdown.props('content')).toBe(output);
    } else {
      expect(dispatched).toEqual({ kind: 'markdown', text: output });
    }

    wrapper.unmount();
  });

  it('renders JSON object node output via the unified dispatcher, not raw pre', () => {
    const output = '{"ok":true,"items":[1,2]}';
    const wrapper = mountContent({ nodeOutput: output });
    const dispatched = resourcePreviewProps(wrapper);

    expect(wrapper.find('[data-accordion-item="output"] pre').exists()).toBe(false);
    expect(dispatched?.kind).toBe('json');
    expect(dispatched).toMatchObject({
      kind: 'json',
      pretty: '{\n  "ok": true,\n  "items": [\n    1,\n    2\n  ]\n}',
    });

    wrapper.unmount();
  });

  it('renders tool provenance args via the unified dispatcher, not JSON.stringify into pre', () => {
    const toolProvenance: ToolProvenance = {
      toolName: 'maenifold.example',
      resolvedArgs: { path: 'memory://note', includeContent: true },
      responseSummary: null,
    };
    const wrapper = mountContent({ toolProvenance });
    const previews = wrapper.findAllComponents({ name: 'ResourcePreview' });
    const dispatched = previews.map((preview) => preview.props('dispatched') as DispatchedResource);

    expect(wrapper.find('[data-accordion-item="tool-provenance"] pre').exists()).toBe(false);
    expect(dispatched.some((resource) => resource.kind === 'json'
      && resource.pretty.includes('"path": "memory://note"'))).toBe(true);

    wrapper.unmount();
  });

  it('empty output region renders cleanly', () => {
    const wrapper = mountContent();

    expect(wrapper.find('[data-testid="accordion"]').exists()).toBe(false);
    expect(wrapper.findComponent({ name: 'MarkdownView' }).exists()).toBe(false);
    expect(wrapper.findComponent({ name: 'ResourcePreview' }).exists()).toBe(false);
    expect(wrapper.find('pre').exists()).toBe(false);
    expect(wrapper.text()).toContain("This node hasn't produced any output yet.");

    wrapper.unmount();
  });
});
