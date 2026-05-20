// @vitest-environment jsdom
/**
 * TDD failing tests for OnboardingStepDone.vue (T-202.4)
 *
 * These tests verify the CORRECT behavior that the component SHOULD have:
 * 1. No /health endpoint calls - should use configStore instead
 * 2. Uses configStore.reload() or configStore.loadConfig() to probe daemon
 * 3. No hardcoded memory path - should display configStore.configPath
 * 4. Shows real config path from store
 * 5. Completes onboarding when configStore succeeds
 *
 * Expected: All tests FAIL against the current buggy implementation.
 */
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { flushPromises, mount } from '@vue/test-utils';
import { ref } from 'vue';

import OnboardingStepDone from '../OnboardingStepDone.vue';
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
  const goBack = vi.fn();
  const markComplete = vi.fn();
  const modelSkipped = ref(false);
  const selectedModelName = ref<string | null>('gemma3:4b');

  return { pinia, configStore, goBack, markComplete, modelSkipped, selectedModelName };
}

describe('OnboardingStepDone', () => {
  describe('no /health endpoint', () => {
    it('does NOT call fetch with /health endpoint', async () => {
      const { pinia, goBack, markComplete, modelSkipped, selectedModelName, configStore } = createTestEnv();

      // Setup configStore to succeed
      vi.spyOn(configStore, 'reload').mockResolvedValue();
      vi.spyOn(configStore, 'loadConfig').mockResolvedValue();
      configStore.error = null;

      // Mock fetch to track calls
      mockFetch.mockResolvedValue({ ok: true });

      const wrapper = mount(OnboardingStepDone, {
        global: {
          plugins: [pinia],
          provide: {
            'onboarding-go-back': goBack,
            'onboarding-mark-complete': markComplete,
            'onboarding-model-skipped': modelSkipped,
            'onboarding-selected-model': selectedModelName,
          },
        },
      });

      // Click "Start using maenifold" button
      const startButton = wrapper.findAll('button').find((b) => b.text().includes('Start using maenifold'));
      expect(startButton).toBeDefined();
      await startButton!.trigger('click');

      await flushPromises();

      // BUG: Component currently calls fetch(`${DAEMON_BASE_URL}/health`)
      // Verify no fetch calls to /health endpoint
      const healthCalls = mockFetch.mock.calls.filter(
        (call) => typeof call[0] === 'string' && call[0].includes('/health')
      );
      expect(healthCalls).toHaveLength(0);
    });
  });

  describe('uses configStore for probe', () => {
    it('calls configStore.reload() or configStore.loadConfig() when finish button is clicked', async () => {
      const { pinia, goBack, markComplete, modelSkipped, selectedModelName, configStore } = createTestEnv();

      const reloadSpy = vi.spyOn(configStore, 'reload').mockResolvedValue();
      const loadConfigSpy = vi.spyOn(configStore, 'loadConfig').mockResolvedValue();
      configStore.error = null;

      // Prevent fetch from being used (should use configStore instead)
      mockFetch.mockRejectedValue(new Error('Should not call fetch'));

      const wrapper = mount(OnboardingStepDone, {
        global: {
          plugins: [pinia],
          provide: {
            'onboarding-go-back': goBack,
            'onboarding-mark-complete': markComplete,
            'onboarding-model-skipped': modelSkipped,
            'onboarding-selected-model': selectedModelName,
          },
        },
      });

      // Click "Start using maenifold" button
      const startButton = wrapper.findAll('button').find((b) => b.text().includes('Start using maenifold'));
      expect(startButton).toBeDefined();
      await startButton!.trigger('click');

      await flushPromises();

      // CORRECT behavior: should call configStore.reload() or configStore.loadConfig()
      const storeProbed = reloadSpy.mock.calls.length > 0 || loadConfigSpy.mock.calls.length > 0;
      expect(storeProbed).toBe(true);
    });
  });

  describe('no hardcoded memory path', () => {
    it('does NOT display the hardcoded literal ~/maenifold/memory', async () => {
      const { pinia, goBack, markComplete, modelSkipped, selectedModelName, configStore } = createTestEnv();

      // Setup a real config path
      configStore.configPath = '/Users/testuser/.config/maenifold/maenifold.json';
      configStore.config = {
        mcpServers: {},
        agents: { architect: { command: 'arch' } },
      };

      const wrapper = mount(OnboardingStepDone, {
        global: {
          plugins: [pinia],
          provide: {
            'onboarding-go-back': goBack,
            'onboarding-mark-complete': markComplete,
            'onboarding-model-skipped': modelSkipped,
            'onboarding-selected-model': selectedModelName,
          },
        },
      });

      await flushPromises();

      // BUG: Component has `const memoryPath = '~/maenifold/memory'` hardcoded
      // The component should NOT contain this hardcoded path
      expect(wrapper.text()).not.toContain('~/maenifold/memory');
    });
  });

  describe('shows real config path', () => {
    it('displays the configStore.configPath value', async () => {
      const { pinia, goBack, markComplete, modelSkipped, selectedModelName, configStore } = createTestEnv();

      // Setup a real config path
      const realConfigPath = '/Users/testuser/.config/maenifold/maenifold.json';
      configStore.configPath = realConfigPath;
      configStore.config = {
        mcpServers: {},
        agents: { architect: { command: 'arch' } },
      };

      const wrapper = mount(OnboardingStepDone, {
        global: {
          plugins: [pinia],
          provide: {
            'onboarding-go-back': goBack,
            'onboarding-mark-complete': markComplete,
            'onboarding-model-skipped': modelSkipped,
            'onboarding-selected-model': selectedModelName,
          },
        },
      });

      await flushPromises();

      // CORRECT behavior: should display the real config path from configStore
      expect(wrapper.text()).toContain(realConfigPath);
    });
  });

  describe('completes onboarding on success', () => {
    it('calls markComplete() when configStore probe succeeds', async () => {
      const { pinia, goBack, markComplete, modelSkipped, selectedModelName, configStore } = createTestEnv();

      // Setup configStore to succeed
      vi.spyOn(configStore, 'reload').mockResolvedValue();
      vi.spyOn(configStore, 'loadConfig').mockResolvedValue();
      configStore.error = null;

      // Also mock fetch to succeed (for current buggy implementation)
      mockFetch.mockResolvedValue({ ok: true });

      const wrapper = mount(OnboardingStepDone, {
        global: {
          plugins: [pinia],
          provide: {
            'onboarding-go-back': goBack,
            'onboarding-mark-complete': markComplete,
            'onboarding-model-skipped': modelSkipped,
            'onboarding-selected-model': selectedModelName,
          },
        },
      });

      // Click "Start using maenifold" button
      const startButton = wrapper.findAll('button').find((b) => b.text().includes('Start using maenifold'));
      expect(startButton).toBeDefined();
      await startButton!.trigger('click');

      await flushPromises();

      // Should call markComplete when daemon probe succeeds
      // Note: This test may pass with the current buggy implementation if fetch succeeds
      // But combined with the "uses configStore" test, it validates the correct behavior
      expect(markComplete).toHaveBeenCalled();
    });

    it('does NOT call markComplete() when configStore probe fails', async () => {
      const { pinia, goBack, markComplete, modelSkipped, selectedModelName, configStore } = createTestEnv();

      // Setup configStore to fail
      vi.spyOn(configStore, 'reload').mockRejectedValue(new Error('Connection refused'));
      vi.spyOn(configStore, 'loadConfig').mockRejectedValue(new Error('Connection refused'));
      configStore.error = 'Connection refused';

      // Also mock fetch to fail (for current buggy implementation)
      mockFetch.mockResolvedValue({ ok: false });

      const wrapper = mount(OnboardingStepDone, {
        global: {
          plugins: [pinia],
          provide: {
            'onboarding-go-back': goBack,
            'onboarding-mark-complete': markComplete,
            'onboarding-model-skipped': modelSkipped,
            'onboarding-selected-model': selectedModelName,
          },
        },
      });

      // Click "Start using maenifold" button
      const startButton = wrapper.findAll('button').find((b) => b.text().includes('Start using maenifold'));
      expect(startButton).toBeDefined();
      await startButton!.trigger('click');

      await flushPromises();

      // Should NOT call markComplete when daemon probe fails
      expect(markComplete).not.toHaveBeenCalled();
    });
  });
});
