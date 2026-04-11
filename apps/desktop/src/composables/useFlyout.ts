import { ref } from 'vue';

export type FlyoutId = 'pipelines' | null;

export function useFlyout() {
  const activeFlyout = ref<FlyoutId>(null);

  function open(id: FlyoutId) {
    activeFlyout.value = id;
  }

  function close() {
    activeFlyout.value = null;
  }

  function toggle(id: NonNullable<FlyoutId>) {
    activeFlyout.value = activeFlyout.value === id ? null : id;
  }

  return { activeFlyout, open, close, toggle };
}
