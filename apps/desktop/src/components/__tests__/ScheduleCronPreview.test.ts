// @vitest-environment jsdom
import { describe, expect, it, vi } from 'vitest';
import { mount } from '@vue/test-utils';
import ScheduleCronPreview from '../ScheduleCronPreview.vue';

const mockState = vi.hoisted(() => ({
  loading: false,
  error: '',
  cron: '0 9 * * 1-5',
  description: 'At 09:00 AM, Monday through Friday',
  nextFires: ['2026-04-29T14:00:00Z', '2026-04-30T14:00:00Z'],
}));

vi.mock('@/composables/useSchedulePreview', async () => {
  const { computed, ref } = await import('vue');
  return {
    useSchedulePreview: () => {
      const loading = ref(mockState.loading);
      const error = ref(mockState.error);
      const nextFires = ref(mockState.nextFires);
      return {
        loading,
        error,
        cron: ref(mockState.cron),
        description: ref(mockState.description),
        nextFires,
        previewIsValid: computed(() => !loading.value && !error.value && nextFires.value.length > 0),
      };
    },
  };
});

function resetState(): void {
  mockState.loading = false;
  mockState.error = '';
  mockState.cron = '0 9 * * 1-5';
  mockState.description = 'At 09:00 AM, Monday through Friday';
  mockState.nextFires = ['2026-04-29T14:00:00Z', '2026-04-30T14:00:00Z'];
}

describe('ScheduleCronPreview', () => {
  it('renders loading state', () => {
    resetState();
    mockState.loading = true;

    const wrapper = mount(ScheduleCronPreview, { props: { expression: 'every weekday at 9am' } });

    expect(wrapper.text()).toContain('Validating...');
  });

  it('renders error state', () => {
    resetState();
    mockState.error = 'invalid cron';
    mockState.nextFires = [];

    const wrapper = mount(ScheduleCronPreview, { props: { expression: 'bad schedule' } });

    expect(wrapper.text()).toContain('invalid cron');
  });

  it('renders valid preview details', () => {
    resetState();

    const wrapper = mount(ScheduleCronPreview, { props: { expression: 'every weekday at 9am' } });

    expect(wrapper.text()).toContain('0 9 * * 1-5');
    expect(wrapper.text()).toContain('At 09:00 AM, Monday through Friday');
    expect(wrapper.text()).toContain('Next 5 fires (Local TZ)');
  });

  it('toggles between local and UTC labels', async () => {
    resetState();
    const wrapper = mount(ScheduleCronPreview, { props: { expression: 'every weekday at 9am' } });

    expect(wrapper.text()).toContain('Show UTC');
    await wrapper.get('button').trigger('click');

    expect(wrapper.text()).toContain('Next 5 fires (UTC)');
    expect(wrapper.text()).toContain('Show local');
    expect(wrapper.text()).toContain('Timezone: UTC');
  });
});
