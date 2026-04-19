import { ref } from 'vue';
import { defineStore } from 'pinia';

export const useFolderPickerStore = defineStore('folderPicker', () => {
  const isOpen = ref(false);
  const initialPath = ref('');
  const resolve = ref<((value: string | null) => void) | null>(null);
  const reject = ref<((reason?: unknown) => void) | null>(null);

  function close(): void {
    isOpen.value = false;
    initialPath.value = '';
    resolve.value = null;
    reject.value = null;
  }

  async function requestPath(nextInitialPath = ''): Promise<string | null> {
    if (resolve.value) {
      resolve.value(null);
    }

    initialPath.value = nextInitialPath;
    isOpen.value = true;

    return new Promise<string | null>((nextResolve, nextReject) => {
      resolve.value = nextResolve;
      reject.value = nextReject;
    });
  }

  function confirm(path: string): void {
    if (resolve.value) {
      resolve.value(path);
    }
    close();
  }

  function cancel(): void {
    if (resolve.value) {
      resolve.value(null);
    }
    close();
  }

  return {
    isOpen,
    initialPath,
    resolve,
    reject,
    requestPath,
    confirm,
    cancel,
  };
});
