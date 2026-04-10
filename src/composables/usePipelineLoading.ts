import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import { storeToRefs } from 'pinia';
import { usePipelineStore } from '../stores/pipeline';

/**
 * Manages initial data loading, connection, and load-failure state for PipelineView.
 */
export function usePipelineLoading() {
  const store = usePipelineStore();
  const { connected, loading } = storeToRefs(store);

  const initialLoadComplete = ref(false);
  const initialLoadFailed = ref(false);
  const connectionError = ref('');

  const showLoadingState = computed(() => !initialLoadComplete.value && loading.value);
  const showErrorState = computed(() => initialLoadFailed.value && !loading.value);

  async function loadInitialData() {
    initialLoadFailed.value = false;
    connectionError.value = '';
    store.clearError();

    try {
      if (!connected.value) {
        await store.connect();
      }

      await Promise.all([
        store.refreshList(),
        store.refreshWorkflows(),
        store.refreshTools(),
      ]);

      initialLoadComplete.value = true;
    } catch (cause) {
      initialLoadFailed.value = true;
      connectionError.value = cause instanceof Error
        ? cause.message
        : 'Failed to connect to the pipeline service.';
    }
  }

  async function handleRetryLoad() {
    await loadInitialData();
  }

  onMounted(async () => {
    await loadInitialData();
  });

  onBeforeUnmount(() => {
    store.disconnect();
  });

  return {
    connectionError,
    showLoadingState,
    showErrorState,
    handleRetryLoad,
  };
}
