import { defineStore } from 'pinia';
import { ref } from 'vue';

export type ToastLevel = 'info' | 'warning' | 'error' | 'success';

export interface Toast {
  id: string;
  message: string;
  level: ToastLevel;
  dismissable: boolean;
  ttl: number;
}

type ToastInput = {
  id?: string;
  message: string;
  level?: ToastLevel;
  dismissable?: boolean;
  ttl?: number;
};

const DEFAULT_TTL_BY_LEVEL: Record<ToastLevel, number> = {
  info: 3_000,
  warning: 5_000,
  error: 0,
  success: 3_000,
};

function createToastId(): string {
  return `toast-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`;
}

export const useToastStore = defineStore('toast', () => {
  const toasts = ref<Toast[]>([]);
  const timers = new Map<string, ReturnType<typeof setTimeout>>();

  function clearTimer(id: string) {
    const timer = timers.get(id);
    if (!timer) {
      return;
    }

    clearTimeout(timer);
    timers.delete(id);
  }

  function removeToast(id: string) {
    clearTimer(id);
    toasts.value = toasts.value.filter((toast) => toast.id !== id);
  }

  function addToast(input: ToastInput): string {
    const level = input.level ?? 'info';
    const ttl = input.ttl ?? DEFAULT_TTL_BY_LEVEL[level];
    const existing = toasts.value.find(
      (toast) => toast.message === input.message && toast.level === level,
    );
    const id = input.id ?? existing?.id ?? createToastId();
    const nextToast: Toast = {
      id,
      message: input.message,
      level,
      dismissable: input.dismissable ?? true,
      ttl,
    };

    clearTimer(id);
    toasts.value = [
      ...toasts.value.filter((toast) => toast.id !== id),
      nextToast,
    ];

    if (ttl > 0) {
      timers.set(id, setTimeout(() => removeToast(id), ttl));
    }

    return id;
  }

  return {
    toasts,
    addToast,
    removeToast,
  };
});
