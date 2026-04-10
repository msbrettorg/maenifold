import { onBeforeUnmount, onMounted, ref, watch } from 'vue';
import type { Ref } from 'vue';

/**
 * Manages responsive/mobile viewport detection and panel visibility for PipelineView.
 *
 * @param hasInspectorSelection - Reactive boolean indicating whether a node/edge is selected.
 */
export function usePipelineMobile(hasInspectorSelection: Ref<boolean>) {
  const isNarrowViewport = ref(false);
  const mobileSidebarOpen = ref(false);
  const mobileInspectorOpen = ref(false);

  let viewportMediaQuery: MediaQueryList | null = null;

  function handleViewportChange(event: MediaQueryList | MediaQueryListEvent) {
    isNarrowViewport.value = event.matches;
    if (!isNarrowViewport.value) {
      mobileSidebarOpen.value = false;
      mobileInspectorOpen.value = false;
      return;
    }

    mobileInspectorOpen.value = hasInspectorSelection.value;
  }

  function toggleMobileSidebar() {
    mobileSidebarOpen.value = !mobileSidebarOpen.value;
    if (mobileSidebarOpen.value) {
      mobileInspectorOpen.value = false;
    }
  }

  function toggleMobileInspector() {
    if (!hasInspectorSelection.value) return;

    mobileInspectorOpen.value = !mobileInspectorOpen.value;
    if (mobileInspectorOpen.value) {
      mobileSidebarOpen.value = false;
    }
  }

  onMounted(() => {
    viewportMediaQuery = window.matchMedia('(max-width: 900px)');
    handleViewportChange(viewportMediaQuery);
    viewportMediaQuery.addEventListener('change', handleViewportChange);
  });

  onBeforeUnmount(() => {
    viewportMediaQuery?.removeEventListener('change', handleViewportChange);
  });

  watch(hasInspectorSelection, (hasSelection) => {
    if (!isNarrowViewport.value) return;
    mobileInspectorOpen.value = hasSelection;
  });

  return {
    isNarrowViewport,
    mobileSidebarOpen,
    mobileInspectorOpen,
    toggleMobileSidebar,
    toggleMobileInspector,
  };
}
