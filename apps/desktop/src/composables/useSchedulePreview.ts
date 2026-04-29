import { computed, getCurrentInstance, onBeforeUnmount, readonly, ref, unref, watch, type Ref } from 'vue';
import { useMcpClient } from '@/composables/useMcpClient';

export interface SchedulePreview {
  action: 'preview';
  cron: string;
  description: string;
  nextFires: string[];
  timezone: string;
}

type SchedulePreviewSource = Ref<string> | string;

export function useSchedulePreview(source: SchedulePreviewSource, delayMs = 300) {
  const { callTool, connect, connected } = useMcpClient();
  const loading = ref(false);
  const error = ref('');
  const cron = ref('');
  const description = ref('');
  const nextFires = ref<string[]>([]);
  const timezone = ref('');
  let timer: ReturnType<typeof setTimeout> | null = null;
  let requestId = 0;

  const previewIsValid = computed(() => !loading.value && !error.value && nextFires.value.length > 0);

  function clearTimer(): void {
    if (timer) {
      clearTimeout(timer);
      timer = null;
    }
  }

  function clearPreview(): void {
    error.value = '';
    cron.value = '';
    description.value = '';
    nextFires.value = [];
    timezone.value = '';
  }

  async function loadPreview(expression: string): Promise<void> {
    const currentRequest = ++requestId;
    loading.value = true;
    error.value = '';
    try {
      if (!connected.value) await connect();
      const result = await callTool<SchedulePreview>('Schedule', {
        action: 'preview',
        schedule: expression,
      });
      if (currentRequest !== requestId) return;
      cron.value = result.cron;
      description.value = result.description;
      nextFires.value = Array.isArray(result.nextFires) ? result.nextFires : [];
      timezone.value = result.timezone;
    } catch (previewError) {
      if (currentRequest !== requestId) return;
      clearPreview();
      error.value = previewError instanceof Error ? previewError.message : String(previewError);
    } finally {
      if (currentRequest === requestId) loading.value = false;
    }
  }

  watch(
    () => unref(source).trim(),
    (expression) => {
      clearTimer();
      requestId += 1;
      loading.value = false;
      clearPreview();
      if (!expression) return;
      loading.value = true;
      timer = setTimeout(() => {
        timer = null;
        void loadPreview(expression);
      }, delayMs);
    },
    { immediate: true },
  );

  if (getCurrentInstance()) {
    onBeforeUnmount(clearTimer);
  }

  return {
    loading: readonly(loading),
    error: readonly(error),
    cron: readonly(cron),
    description: readonly(description),
    nextFires: readonly(nextFires),
    timezone: readonly(timezone),
    previewIsValid,
  };
}
