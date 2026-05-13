// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { defineComponent } from 'vue';

type UserPromptPayload = {
  id: string;
  kind: 'elicitation';
  payload: { message?: unknown; requestedSchema?: unknown };
  timeoutMs: number;
};

function promptPayload(schema: Record<string, unknown>): UserPromptPayload {
  return {
    id: 'prompt-1',
    kind: 'elicitation',
    payload: {
      message: 'Provide deployment reason',
      requestedSchema: schema,
    },
    timeoutMs: 60_000,
  };
}

async function setup(schema: Record<string, unknown>) {
  vi.resetModules();
  setActivePinia(createPinia());
  const resolveUserPrompt = vi.fn().mockResolvedValue({ ok: true });
  const listeners: Array<(payload: UserPromptPayload) => void> = [];
  Object.defineProperty(window, 'api', {
    value: {
      quit: vi.fn(),
      onUserPromptRequest: vi.fn((handler: (payload: UserPromptPayload) => void) => {
        listeners.push(handler);
        return vi.fn();
      }),
      resolveUserPrompt,
    },
    configurable: true,
  });
  const [{ default: ElicitationModal }, { useUserPromptStore }] = await Promise.all([
    import('../ElicitationModal.vue'),
    import('../../stores/user-prompt'),
  ]);
  const store = useUserPromptStore();
  store.setupIpcListener();
  const listener = listeners[0];
  if (!listener) throw new Error('user prompt listener was not registered');
  listener(promptPayload(schema));
  const wrapper = mount(ElicitationModal, {
    global: {
      stubs: {
        Dialog: defineComponent({
          props: { open: Boolean },
          template: '<section v-if="open"><slot /></section>',
        }),
        DialogContent: { template: '<div><slot /></div>' },
        DialogFooter: { template: '<footer><slot /></footer>' },
        DialogHeader: { template: '<header><slot /></header>' },
        DialogTitle: { template: '<h2><slot /></h2>' },
        Button: { template: '<button type="button"><slot /></button>' },
        Textarea: {
          props: { modelValue: String },
          emits: ['update:modelValue'],
          methods: {
            handleInput(event: Event) {
              this.$emit('update:modelValue', (event.target as HTMLTextAreaElement).value);
            },
          },
          template: '<textarea :value="modelValue" @input="handleInput" />',
        },
        SchemaFormRenderer: {
          name: 'SchemaFormRenderer',
          props: ['schema', 'data', 'ariaLabel'],
          emits: ['change'],
          template: '<button type="button" data-testid="schema-change" @click="$emit(\'change\', { data: { reason: \'approved\' } })">change</button>',
        },
      },
    },
  });
  return { resolveUserPrompt, wrapper };
}

describe('ElicitationModal', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    Reflect.deleteProperty(window, 'api');
    vi.spyOn(Date, 'now').mockReturnValue(1_700_000_000_000);
  });

  it('renders the MCP message and passes the requested schema to SchemaFormRenderer', async () => {
    const schema = {
      type: 'object',
      properties: { reason: { type: 'string' } },
      required: ['reason'],
    };
    const { wrapper } = await setup(schema);

    expect(wrapper.text()).toContain('Provide deployment reason');
    expect(wrapper.getComponent({ name: 'SchemaFormRenderer' }).props('schema')).toEqual(schema);
  });

  it('submits changed form values through the user prompt store', async () => {
    const { resolveUserPrompt, wrapper } = await setup({
      type: 'object',
      properties: { reason: { type: 'string' } },
    });

    await wrapper.get('[data-testid="schema-change"]').trigger('click');
    await wrapper.findAll('button').find((button) => button.text() === 'Submit')!.trigger('click');

    expect(resolveUserPrompt).toHaveBeenCalledWith({
      kind: 'elicitation',
      id: 'prompt-1',
      response: { action: 'accept', content: { reason: 'approved' } },
    });
  });

  it('cancels the active prompt from the cancel button', async () => {
    const { resolveUserPrompt, wrapper } = await setup({
      type: 'object',
      properties: { reason: { type: 'string' } },
    });

    await wrapper.findAll('button').find((button) => button.text() === 'Cancel')!.trigger('click');

    expect(resolveUserPrompt).toHaveBeenCalledWith({
      kind: 'elicitation',
      id: 'prompt-1',
      response: { action: 'cancel' },
    });
  });

  it('uses raw JSON fallback for unsupported schemas and submits parsed content', async () => {
    const { resolveUserPrompt, wrapper } = await setup({
      type: 'object',
      patternProperties: { '^x-': { type: 'string' } },
    });

    expect(wrapper.text()).toContain('enter JSON directly');
    await wrapper.get('textarea').setValue('{"x-reason":"manual"}');
    await wrapper.findAll('button').find((button) => button.text() === 'Submit')!.trigger('click');

    expect(resolveUserPrompt).toHaveBeenCalledWith({
      kind: 'elicitation',
      id: 'prompt-1',
      response: { action: 'accept', content: { 'x-reason': 'manual' } },
    });
  });
});
