// @vitest-environment jsdom
/**
 * TDD failing tests for OnboardingStepAgent.vue (T-202.3)
 *
 * BUG: Component has hardcoded runtime presets (copilot-cli, claude, openai)
 * at lines 15-37 that don't match how agents work. Agents use maenifold.json
 * with a `command` field for arbitrary shell commands.
 *
 * Reference:
 * - AgentConfig = { command: string; env?; mcpServers? } (types.ts)
 * - SettingsConnectionAgents.vue uses free-form command input with placeholder:
 *   "npx @github/copilot-language-server@latest --acp"
 *
 * Expected: All tests FAIL against the current buggy implementation.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { flushPromises, mount } from '@vue/test-utils';

import OnboardingStepAgent from '../OnboardingStepAgent.vue';
import { useConfigStore } from '../../stores/config';

beforeEach(() => {
  vi.useFakeTimers();
  vi.restoreAllMocks();
});

afterEach(() => {
  vi.useRealTimers();
});

function createTestEnv() {
  const pinia = createPinia();
  setActivePinia(pinia);

  const configStore = useConfigStore();

  // Provide onboarding inject keys
  const advance = vi.fn();
  const goBack = vi.fn();

  return { pinia, configStore, advance, goBack };
}

describe('OnboardingStepAgent', () => {
  describe('no hardcoded presets', () => {
    it('does NOT display "GitHub Copilot CLI" preset option', async () => {
      const { pinia, advance, goBack, configStore } = createTestEnv();

      vi.spyOn(configStore, 'addAgent').mockResolvedValue();

      const wrapper = mount(OnboardingStepAgent, {
        global: {
          plugins: [pinia],
          provide: {
            'onboarding-advance': advance,
            'onboarding-go-back': goBack,
          },
        },
      });

      await flushPromises();

      // BUG: Component shows hardcoded "GitHub Copilot CLI" preset
      // CORRECT: Should have free-form command input, no presets
      expect(wrapper.text()).not.toContain('GitHub Copilot CLI');
    });

    it('does NOT display "Claude (Anthropic)" preset option', async () => {
      const { pinia, advance, goBack, configStore } = createTestEnv();

      vi.spyOn(configStore, 'addAgent').mockResolvedValue();

      const wrapper = mount(OnboardingStepAgent, {
        global: {
          plugins: [pinia],
          provide: {
            'onboarding-advance': advance,
            'onboarding-go-back': goBack,
          },
        },
      });

      await flushPromises();

      // BUG: Component shows hardcoded "Claude (Anthropic)" preset
      // CORRECT: Should have free-form command input, no presets
      expect(wrapper.text()).not.toContain('Claude (Anthropic)');
    });

    it('does NOT display "OpenAI GPT" preset option', async () => {
      const { pinia, advance, goBack, configStore } = createTestEnv();

      vi.spyOn(configStore, 'addAgent').mockResolvedValue();

      const wrapper = mount(OnboardingStepAgent, {
        global: {
          plugins: [pinia],
          provide: {
            'onboarding-advance': advance,
            'onboarding-go-back': goBack,
          },
        },
      });

      await flushPromises();

      // BUG: Component shows hardcoded "OpenAI GPT" preset
      // CORRECT: Should have free-form command input, no presets
      expect(wrapper.text()).not.toContain('OpenAI GPT');
    });
  });

  describe('free-form command input', () => {
    it('has a text input for agent command (not radio buttons)', async () => {
      const { pinia, advance, goBack, configStore } = createTestEnv();

      vi.spyOn(configStore, 'addAgent').mockResolvedValue();

      const wrapper = mount(OnboardingStepAgent, {
        global: {
          plugins: [pinia],
          provide: {
            'onboarding-advance': advance,
            'onboarding-go-back': goBack,
          },
        },
      });

      await flushPromises();

      // Find a text input for command entry
      // BUG: Component uses radio-button-style preset buttons, not a command input
      const commandInput = wrapper.find('input[type="text"][placeholder*="copilot"]');
      expect(commandInput.exists()).toBe(true);
    });

    it('command input has correct ACP placeholder', async () => {
      const { pinia, advance, goBack, configStore } = createTestEnv();

      vi.spyOn(configStore, 'addAgent').mockResolvedValue();

      const wrapper = mount(OnboardingStepAgent, {
        global: {
          plugins: [pinia],
          provide: {
            'onboarding-advance': advance,
            'onboarding-go-back': goBack,
          },
        },
      });

      await flushPromises();

      // Find all text inputs and check for one with a real ACP command placeholder
      // Reference: SettingsConnectionAgents.vue uses "npx @github/copilot-language-server@latest --acp"
      const inputs = wrapper.findAll('input[type="text"]');
      const commandInput = inputs.find((input) => {
        const placeholder = input.attributes('placeholder') ?? '';
        // Should contain a real ACP command, not just "Agent name" or a preset ID
        return (
          placeholder.includes('--acp') ||
          placeholder.includes('copilot-language-server') ||
          placeholder.includes('npx')
        );
      });

      // BUG: No such input exists - component uses preset buttons
      expect(commandInput).toBeDefined();
      expect(commandInput?.attributes('placeholder')).toContain('--acp');
    });
  });

  describe('addAgent uses command string', () => {
    it('calls addAgent with user-typed command, not preset ID', async () => {
      const { pinia, advance, goBack, configStore } = createTestEnv();

      const addAgentSpy = vi.spyOn(configStore, 'addAgent').mockResolvedValue();

      const wrapper = mount(OnboardingStepAgent, {
        global: {
          plugins: [pinia],
          provide: {
            'onboarding-advance': advance,
            'onboarding-go-back': goBack,
          },
        },
      });

      await flushPromises();

      // Find the name input and set agent name
      const nameInput = wrapper.find('#agent-name');
      expect(nameInput.exists()).toBe(true);
      await nameInput.setValue('MyAgent');

      // Find command input and set a real ACP command
      // BUG: This input doesn't exist in the current implementation
      const inputs = wrapper.findAll('input[type="text"]');
      const commandInput = inputs.find((input) => {
        const placeholder = input.attributes('placeholder') ?? '';
        const id = input.attributes('id') ?? '';
        // Look for command input (not the name input)
        return id !== 'agent-name' && (
          placeholder.includes('command') ||
          placeholder.includes('--acp') ||
          placeholder.includes('npx')
        );
      });

      // If command input exists, set a real command
      if (commandInput) {
        await commandInput.setValue('npx @github/copilot-language-server@latest --acp');
      }

      // Advance past the CLI preflight debounce (500ms) so checking resolves
      vi.advanceTimersByTime(600);
      await flushPromises();

      // Submit the form
      const buttons = wrapper.findAll('button');
      const addButton = buttons.find((b) => b.text().includes('Add agent'));
      expect(addButton).toBeDefined();
      await addButton!.trigger('click');

      await flushPromises();

      // Verify addAgent was called with the correct shape
      expect(addAgentSpy).toHaveBeenCalled();

      const [, agentConfig] = addAgentSpy.mock.calls[0];

      // BUG: Current implementation passes { command: 'copilot-cli' } (preset ID)
      // CORRECT: Should pass { command: '<user-typed-command>' }
      expect(agentConfig.command).not.toBe('copilot-cli');
      expect(agentConfig.command).not.toBe('claude');
      expect(agentConfig.command).not.toBe('openai');

      // Command should be a real shell command, not a preset identifier
      expect(agentConfig.command).toMatch(/npx|node|--|\.js/);
    });
  });

  describe('skip when agents exist', () => {
    it('shows existing agents when configStore.hasAgents is true', async () => {
      const { pinia, advance, goBack, configStore } = createTestEnv();

      // Simulate existing agents in config
      configStore.config = {
        mcpServers: {},
        agents: {
          'existing-agent': { command: 'npx some-agent --acp' },
        },
      };

      vi.spyOn(configStore, 'addAgent').mockResolvedValue();

      const wrapper = mount(OnboardingStepAgent, {
        global: {
          plugins: [pinia],
          provide: {
            'onboarding-advance': advance,
            'onboarding-go-back': goBack,
          },
        },
      });

      await flushPromises();

      // When agents already exist, component should acknowledge them
      // BUG: Component always shows "Add your first agent" regardless
      // CORRECT: Should show existing agents or skip message

      // Check that hasAgents is true
      expect(configStore.hasAgents).toBe(true);
      expect(configStore.agentNames).toContain('existing-agent');

      // Component should indicate that agents exist (not "Add your first agent")
      // This could be:
      // - Showing the existing agent names
      // - Showing "You have 1 agent configured"
      // - Showing a skip option
      const text = wrapper.text();

      // Should NOT say "first agent" when agents exist
      const showsFirstAgentMessage = text.includes('Add your first agent');

      // Either show existing agents or don't show "first agent" text
      const acknowledgesExistingAgents =
        text.includes('existing-agent') ||
        text.includes('1 agent') ||
        text.includes('agents configured') ||
        text.includes('Skip') ||
        !showsFirstAgentMessage;

      expect(acknowledgesExistingAgents).toBe(true);
    });
  });
});
