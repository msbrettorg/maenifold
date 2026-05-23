import {
  toastSuccess as realSuccess,
  toastInfo as realInfo,
  toastWarning as realWarning,
  toastError as realError,
  toastCritical as realCritical,
  type ToastOptions,
} from '@/composables/useToast';

export interface NotificationSink {
  success(message: string, opts?: ToastOptions): void;
  info(message: string, opts?: ToastOptions): void;
  warning(message: string, opts?: ToastOptions): void;
  error(message: string, opts?: ToastOptions): void;
  critical(message: string, opts?: ToastOptions): void;
}

const sonnerSink: NotificationSink = {
  success: realSuccess,
  info: realInfo,
  warning: realWarning,
  error: realError,
  critical: realCritical,
};

let activeSink: NotificationSink = sonnerSink;

export function getNotificationSink(): NotificationSink {
  return activeSink;
}

export function setNotificationSink(sink: NotificationSink): void {
  activeSink = sink;
}

export function resetNotificationSink(): void {
  activeSink = sonnerSink;
}

export function createNoopNotificationSink(): NotificationSink {
  return {
    success: () => {},
    info: () => {},
    warning: () => {},
    error: () => {},
    critical: () => {},
  };
}
