import { toast as sonnerToast } from 'vue-sonner';
import type { ExternalToast } from 'vue-sonner';

export type ToastSeverity = 'success' | 'info' | 'warning' | 'error' | 'critical';
export type ToastOptions = ExternalToast;

export const TOAST_DURATIONS: Record<ToastSeverity, number> = {
  success: 3000,
  info: 3000,
  warning: 5000,
  error: Infinity,
  critical: Infinity,
};

export function toastSuccess(message: string, opts?: ToastOptions): void {
  sonnerToast.success(message, { duration: TOAST_DURATIONS.success, ...opts });
}

export function toastInfo(message: string, opts?: ToastOptions): void {
  sonnerToast.info(message, { duration: TOAST_DURATIONS.info, ...opts });
}

export function toastWarning(message: string, opts?: ToastOptions): void {
  sonnerToast.warning(message, { duration: TOAST_DURATIONS.warning, ...opts });
}

export function toastError(message: string, opts?: ToastOptions): void {
  sonnerToast.error(message, { duration: TOAST_DURATIONS.error, closeButton: true, ...opts });
}

export function toastCritical(message: string, opts?: ToastOptions): void {
  sonnerToast.error(message, { closeButton: true, ...opts, duration: TOAST_DURATIONS.critical });
}
