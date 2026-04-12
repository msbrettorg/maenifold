import { onBeforeUnmount, onMounted, ref } from 'vue';

/**
 * Manages responsive/mobile viewport detection and sidebar visibility for pipeline views.
 */
export function usePipelineMobile() {
  const isNarrowViewport = ref(false);
  const mobileSidebarOpen = ref(false);

  let viewportMediaQuery: MediaQueryList | null = null;

  function handleViewportChange(event: MediaQueryList | MediaQueryListEvent) {
    isNarrowViewport.value = event.matches;
    if (!isNarrowViewport.value) {
      mobileSidebarOpen.value = false;
    }
  }

  function toggleMobileSidebar() {
    mobileSidebarOpen.value = !mobileSidebarOpen.value;
  }

  onMounted(() => {
    viewportMediaQuery = window.matchMedia('(max-width: 900px)');
    handleViewportChange(viewportMediaQuery);
    viewportMediaQuery.addEventListener('change', handleViewportChange);
  });

  onBeforeUnmount(() => {
    viewportMediaQuery?.removeEventListener('change', handleViewportChange);
  });

  return {
    isNarrowViewport,
    mobileSidebarOpen,
    toggleMobileSidebar,
  };
}
