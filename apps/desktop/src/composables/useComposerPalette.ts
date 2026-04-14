import { computed, ref, watch } from 'vue';

import { usePipelineMobile } from './usePipelineMobile';
import { useResizeHandle } from './useResizeHandle';
import { NODE_PALETTE_MIN_WIDTH, useNodePaletteStore } from '../stores/node-palette';

export function useComposerPalette(getDesignMode: () => boolean) {
  const nodePaletteStore = useNodePaletteStore();
  const paletteResize = useResizeHandle({
    initial: nodePaletteStore.panelWidth,
    min: NODE_PALETTE_MIN_WIDTH,
    max: 720,
  });
  const paletteOpen = ref(false);
  const { isNarrowViewport } = usePipelineMobile();

  const showPalettePanel = computed(
    () => getDesignMode() && paletteOpen.value && !isNarrowViewport.value,
  );
  const paletteAsOverlay = computed(
    () => getDesignMode() && isNarrowViewport.value && paletteOpen.value,
  );
  const composerLayoutStyle = computed(() => ({
    'grid-template-columns': showPalettePanel.value
      ? `${paletteResize.width.value}px minmax(0, 1fr)`
      : 'minmax(0, 1fr)',
  }));

  watch(
    () => paletteResize.width.value,
    (value) => {
      nodePaletteStore.setPanelWidth(value);
    },
    { immediate: true },
  );

  function togglePalette(): void {
    paletteOpen.value = !paletteOpen.value;
  }

  function closePalette(): void {
    paletteOpen.value = false;
  }

  return {
    composerLayoutStyle,
    closePalette,
    paletteAsOverlay,
    paletteOpen,
    paletteResize,
    showPalettePanel,
    togglePalette,
  };
}
