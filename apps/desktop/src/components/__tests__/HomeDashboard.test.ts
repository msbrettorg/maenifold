// @vitest-environment jsdom
/**
 * T-243.23 blue-team: Home search affordance
 *
 * Verifies that submitting the Knowledge search input on HomeDashboard:
 *   (a) calls useAppShellStore().setActiveTab('knowledge')
 *   (b) seeds useKnowledgeStore().searchQuery with the typed value
 *   (c) empty / whitespace submit is a no-op (no navigation)
 */
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { ref } from 'vue';
import HomeDashboard from '../HomeDashboard.vue';
import { useAppShellStore } from '../../stores/app-shell';
import { useKnowledgeStore } from '../../stores/knowledge';

vi.mock('../../composables/useDaemonHealth', () => ({
  useDaemonHealth: () => ({
    daemonOnline: ref(true),
    lastErrorMessage: ref(null),
    forceCheck: vi.fn(),
  }),
}));

vi.mock('../../composables/useMemoryApi', () => ({
  useMemoryApi: () => ({
    listMemories: vi.fn().mockResolvedValue([]),
    searchMemories: vi.fn().mockResolvedValue({ results: [] }),
    readMemory: vi.fn(),
    buildContext: vi.fn(),
    findSimilarConcepts: vi.fn(),
    writeMemory: vi.fn(),
    editMemory: vi.fn(),
    deleteMemory: vi.fn(),
    moveMemory: vi.fn(),
  }),
}));

const stubs = {
  HomeRecentActivity: { template: '<div data-stub="recent-activity" />' },
};

const defaultProps = {
  hasAgents: false,
  selectedCwd: '',
  isConnected: false,
  isConnecting: false,
  recentSessions: [],
  submitPrompt: vi.fn().mockResolvedValue(true),
};

describe('HomeDashboard — search affordance (T-243.23)', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it('renders a search input with aria-label "Search Knowledge"', () => {
    const wrapper = mount(HomeDashboard, {
      props: defaultProps,
      global: { stubs },
    });
    const input = wrapper.find('[aria-label="Search Knowledge"]');
    expect(input.exists()).toBe(true);
  });

  it('navigates to knowledge tab and seeds searchQuery on form submit', async () => {
    const wrapper = mount(HomeDashboard, {
      props: defaultProps,
      global: { stubs },
    });

    const appShell = useAppShellStore();
    const knowledge = useKnowledgeStore();

    const input = wrapper.find('[aria-label="Search Knowledge"]');
    await input.setValue('cortex notes');

    const form = wrapper.find('[data-testid="home-search-form"]');
    await form.trigger('submit');

    // (a) tab was switched to knowledge
    expect(appShell.activeTab).toBe('knowledge');
    // (b) knowledge store received the typed query
    expect(knowledge.searchQuery).toBe('cortex notes');
  });

  it('clears the search input after successful submit', async () => {
    const wrapper = mount(HomeDashboard, {
      props: defaultProps,
      global: { stubs },
    });

    const input = wrapper.find('[aria-label="Search Knowledge"]');
    await input.setValue('some query');

    const form = wrapper.find('[data-testid="home-search-form"]');
    await form.trigger('submit');

    expect((input.element as HTMLInputElement).value).toBe('');
  });

  it('is a no-op when input is empty (no navigation, no query seed)', async () => {
    const wrapper = mount(HomeDashboard, {
      props: defaultProps,
      global: { stubs },
    });

    const appShell = useAppShellStore();
    const knowledge = useKnowledgeStore();
    const initialTab = appShell.activeTab;

    const form = wrapper.find('[data-testid="home-search-form"]');
    await form.trigger('submit');

    expect(appShell.activeTab).toBe(initialTab);
    expect(knowledge.searchQuery).toBe('');
  });

  it('is a no-op when input is whitespace-only', async () => {
    const wrapper = mount(HomeDashboard, {
      props: defaultProps,
      global: { stubs },
    });

    const appShell = useAppShellStore();
    const input = wrapper.find('[aria-label="Search Knowledge"]');
    await input.setValue('   ');

    const form = wrapper.find('[data-testid="home-search-form"]');
    await form.trigger('submit');

    expect(appShell.activeTab).not.toBe('knowledge');
  });
});
