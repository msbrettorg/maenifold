// @vitest-environment jsdom
/**
 * TDD failing tests for OnboardingStepDaemon.vue (T-202.2)
 *
 * These tests verify the CORRECT behavior that the component SHOULD have:
 * 1. Command text should be `maenifold --http` (not `maenifold-cli serve`)
 * 2. Health probe should use configStore.loadConfig() (not raw fetch to `/health`)
 * 3. Connected state should show when loadConfig() succeeds
 * 4. Disconnected state should show when loadConfig() fails, with correct command
 *
 * Expected: All tests FAIL against the current buggy implementation.
 */
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { flushPromises, mount } from '@vue/test-utils';

import OnboardingStepDaemon from '../OnboardingStepDaemon.vue';
import { useConfigStore } from '../../stores/config';

// Mock fetch globally to verify it's NOT called for /health
const mockFetch = vi.fn();

beforeEach(() => {
  vi.restoreAllMocks();
  mockFetch.mockReset();
  globalThis.fetch = mockFetch;
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

describe('OnboardingStepDaemon', () => {
  describe('command text', () => {
    it('displays the correct daemon command: maenifold --http', async () => {
      const { pinia, advance, goBack, configStore } = createTestEnv();

      // Make loadConfig fail so we see disconnected state with command
      vi.spyOn(configStore, 'loadConfig').mockRejectedValue(new Error('Connection refused'));
      configStore.error = 'Connection refused';

      const wrapper = mount(OnboardingStepDaemon, {
        global: {
          plugins: [pinia],
          provide: {
            'onboarding-advance': advance,
            'onboarding-go-back': goBack,
          },
        },
      });

      await flushPromises();

      // Find the command display element (the <code> tag in the component)
      const codeElement = wrapper.find('code');
      expect(codeElement.exists()).toBe(true);

      // BUG: Component has `maenifold-cli serve` but should have `maenifold --http`
      expect(codeElement.text()).toBe('maenifold --http');
    });
  });

  describe('health probe mechanism', () => {
    it('uses configStore.loadConfig() instead of raw fetch to /health', async () => {
      const { pinia, advance, goBack, configStore } = createTestEnv();

      const loadConfigSpy = vi.spyOn(configStore, 'loadConfig').mockResolvedValue();

      mount(OnboardingStepDaemon, {
        global: {
          plugins: [pinia],
          provide: {
            'onboarding-advance': advance,
            'onboarding-go-back': goBack,
          },
        },
      });

      await flushPromises();

      // CORRECT behavior: should call configStore.loadConfig()
      expect(loadConfigSpy).toHaveBeenCalled();

      // BUG: Component currently fetches /health directly
      // Verify no fetch calls to /health endpoint
      const healthCalls = mockFetch.mock.calls.filter(
        (call) => typeof call[0] === 'string' && call[0].includes('/health')
      );
      expect(healthCalls).toHaveLength(0);
    });

    it('does not call fictional /health endpoint', async () => {
      const { pinia, advance, goBack, configStore } = createTestEnv();

      // Let the component try its current buggy behavior
      mockFetch.mockResolvedValue({
        ok: true,
        json: () => Promise.resolve({ version: '1.0.0', tools: 5, agents: 2 }),
      });

      vi.spyOn(configStore, 'loadConfig').mockResolvedValue();

      mount(OnboardingStepDaemon, {
        global: {
          plugins: [pinia],
          provide: {
            'onboarding-advance': advance,
            'onboarding-go-back': goBack,
          },
        },
      });

      await flushPromises();

      // BUG: Component fetches /health which doesn't exist
      // The daemon only has /api/config, not /health
      const allFetchUrls = mockFetch.mock.calls.map((call) => call[0]);
      const healthEndpointCalled = allFetchUrls.some(
        (url) => typeof url === 'string' && url.includes('/health')
      );

      // This should be false - no /health calls should be made
      expect(healthEndpointCalled).toBe(false);
    });
  });

  describe('connected state', () => {
    it('shows connected state when configStore.loadConfig() succeeds', async () => {
      const { pinia, advance, goBack, configStore } = createTestEnv();

      // Simulate successful config load
      vi.spyOn(configStore, 'loadConfig').mockImplementation(async () => {
        configStore.config = {
          mcpServers: [],
          agents: { architect: { command: 'architect' } },
        };
        configStore.error = null;
      });

      const wrapper = mount(OnboardingStepDaemon, {
        global: {
          plugins: [pinia],
          provide: {
            'onboarding-advance': advance,
            'onboarding-go-back': goBack,
          },
        },
      });

      await flushPromises();

      // Should show connected state
      expect(wrapper.text()).toContain('Connected');

      // Continue button should be enabled
      const continueButton = wrapper.find('button:last-child');
      expect(continueButton.attributes('disabled')).toBeUndefined();
    });

    it('displays AgentsConfig data (not fictional DaemonStatus)', async () => {
      const { pinia, advance, goBack, configStore } = createTestEnv();

      // Simulate successful config load with real AgentsConfig shape
      vi.spyOn(configStore, 'loadConfig').mockImplementation(async () => {
        configStore.config = {
          mcpServers: [
            { type: 'stdio', name: 'maenifold', command: 'maenifold', args: ['--mcp'] },
          ],
          agents: {
            architect: { command: 'architect' },
            swe: { command: 'swe' },
          },
        };
        configStore.error = null;
      });

      const wrapper = mount(OnboardingStepDaemon, {
        global: {
          plugins: [pinia],
          provide: {
            'onboarding-advance': advance,
            'onboarding-go-back': goBack,
          },
        },
      });

      await flushPromises();

      // Should show connected state with AgentsConfig-derived info
      // NOT the fictional DaemonStatus { version, tools, agents }
      expect(wrapper.text()).toContain('Connected');

      // The component should show agent count from configStore.agentNames
      // NOT from a fictional DaemonStatus.agents number
      expect(configStore.agentNames).toEqual(['architect', 'swe']);
    });
  });

  describe('disconnected state', () => {
    it('shows disconnected state when configStore.loadConfig() fails', async () => {
      const { pinia, advance, goBack, configStore } = createTestEnv();

      // Simulate failed config load
      vi.spyOn(configStore, 'loadConfig').mockImplementation(async () => {
        configStore.error = 'Failed to load config: HTTP 502';
        throw new Error('Connection refused');
      });

      const wrapper = mount(OnboardingStepDaemon, {
        global: {
          plugins: [pinia],
          provide: {
            'onboarding-advance': advance,
            'onboarding-go-back': goBack,
          },
        },
      });

      await flushPromises();

      // Should show disconnected state
      expect(wrapper.text()).toContain('Disconnected');

      // Should show the correct command to start the daemon
      expect(wrapper.text()).toContain('maenifold --http');
    });

    it('shows disconnected with correct command when daemon is unreachable', async () => {
      const { pinia, advance, goBack, configStore } = createTestEnv();

      // Simulate network error
      vi.spyOn(configStore, 'loadConfig').mockRejectedValue(
        new Error('fetch failed')
      );
      configStore.error = 'fetch failed';

      const wrapper = mount(OnboardingStepDaemon, {
        global: {
          plugins: [pinia],
          provide: {
            'onboarding-advance': advance,
            'onboarding-go-back': goBack,
          },
        },
      });

      await flushPromises();

      // Find the command display
      const codeElement = wrapper.find('code');
      expect(codeElement.exists()).toBe(true);

      // BUG: Shows `maenifold-cli serve` but should show `maenifold --http`
      expect(codeElement.text()).toBe('maenifold --http');
    });
  });

  describe('continue button behavior', () => {
    it('enables continue button only when connected via configStore', async () => {
      const { pinia, advance, goBack, configStore } = createTestEnv();

      // Start disconnected
      vi.spyOn(configStore, 'loadConfig').mockRejectedValue(new Error('offline'));
      configStore.error = 'offline';

      const wrapper = mount(OnboardingStepDaemon, {
        global: {
          plugins: [pinia],
          provide: {
            'onboarding-advance': advance,
            'onboarding-go-back': goBack,
          },
        },
      });

      await flushPromises();

      // Continue button should be disabled when disconnected
      const buttons = wrapper.findAll('button');
      const continueButton = buttons.find((b) => b.text().includes('Continue'));
      expect(continueButton).toBeDefined();
      expect(continueButton!.attributes('disabled')).toBeDefined();
    });
  });
});
