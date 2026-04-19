import { ref } from 'vue';
import { defineStore } from 'pinia';

export const useConfirmDialogStore = defineStore('confirmDialog', () => {
  const open = ref(false);
  const title = ref('');
  const description = ref('');
  const resolve = ref<((value: boolean) => void) | null>(null);

  function close(): void {
    open.value = false;
    title.value = '';
    description.value = '';
    resolve.value = null;
  }

  async function confirm(nextTitle: string, nextDescription = ''): Promise<boolean> {
    if (resolve.value) {
      resolve.value(false);
    }

    title.value = nextTitle;
    description.value = nextDescription;
    open.value = true;

    return new Promise<boolean>((nextResolve) => {
      resolve.value = nextResolve;
    });
  }

  function handleConfirm(): void {
    if (resolve.value) {
      resolve.value(true);
    }
    close();
  }

  function handleCancel(): void {
    if (resolve.value) {
      resolve.value(false);
    }
    close();
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
