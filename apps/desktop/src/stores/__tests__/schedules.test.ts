import { beforeEach, describe, expect, it } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { useSchedulesStore } from '@/stores/schedules';

describe('schedules store', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it('pauseState normalization handles missing optional fields', () => {
    const store = useSchedulesStore();

    store.setPauseState({ paused: true });
    expect(store.pauseState).toEqual({ paused: true });
    expect(store.isPausedAll).toBe(true);

    store.setPauseState({});
    expect(store.pauseState).toEqual({ paused: false });
    expect(store.isPausedAll).toBe(false);
  });
});
