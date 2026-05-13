import { computed, ref } from 'vue';
import { defineStore } from 'pinia';

type ConfirmRequest = {
  title: string;
  description: string;
  resolve: (value: boolean) => void;
};

export const useConfirmDialogStore = defineStore('confirmDialog', () => {
  const queue = ref<ConfirmRequest[]>([]);
  const front = computed(() => queue.value[0] ?? null);
  const open = computed(() => queue.value.length > 0);
  const title = computed(() => front.value?.title ?? '');
  const description = computed(() => front.value?.description ?? '');

  function resolveFront(value: boolean): void {
    const request = queue.value.shift();
    if (request) {
      request.resolve(value);
    }
  }

  function confirm(nextTitle: string, nextDescription = ''): Promise<boolean> {
    return new Promise<boolean>((nextResolve) => {
      queue.value.push({
        title: nextTitle,
        description: nextDescription,
        resolve: nextResolve,
      });
    });
  }

  function handleConfirm(): void {
    resolveFront(true);
  }

  function handleCancel(): void {
    resolveFront(false);
  }

  return {
    open,
    title,
    description,
    confirm,
    handleConfirm,
    handleCancel,
  };
});
