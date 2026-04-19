import { ref } from 'vue';
import { defineStore } from 'pinia';

export interface PromptDialogRequest {
  title: string;
  description?: string;
  placeholder?: string;
  confirmLabel?: string;
  initialValue?: string;
}

export const usePromptDialogStore = defineStore('promptDialog', () => {
  const isOpen = ref(false);
  const title = ref('');
  const description = ref('');
  const placeholder = ref('');
  const confirmLabel = ref('Continue');
  const initialValue = ref('');
  const resolve = ref<((value: string | null) => void) | null>(null);

  function close(): void {
    isOpen.value = false;
    title.value = '';
    description.value = '';
    placeholder.value = '';
    confirmLabel.value = 'Continue';
    initialValue.value = '';
    resolve.value = null;
  }

  async function requestValue(request: PromptDialogRequest): Promise<string | null> {
    if (resolve.value) {
      resolve.value(null);
    }

    title.value = request.title;
    description.value = request.description ?? '';
    placeholder.value = request.placeholder ?? '';
    confirmLabel.value = request.confirmLabel?.trim() || 'Continue';
    initialValue.value = request.initialValue ?? '';
    isOpen.value = true;

    return new Promise<string | null>((nextResolve) => {
      resolve.value = nextResolve;
    });
  }

  function confirm(value: string): void {
    if (resolve.value) {
      resolve.value(value);
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
    title,
    description,
    placeholder,
    confirmLabel,
    initialValue,
    requestValue,
    confirm,
    cancel,
  };
});
