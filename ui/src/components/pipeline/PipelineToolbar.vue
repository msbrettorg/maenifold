<script setup lang="ts">
// Pipeline canvas toolbar. Teleported into the shell's `#pipeline-toolbar`
// mount target so it floats over the canvas without the canvas owning it. All
// state/actions flow through the shared canvas controller (no props/emits cross
// the teleport boundary). Vue 3.5 `<Teleport defer>` resolves the target after
// the shell has mounted it.
import { LayoutGrid, Maximize2, MoveHorizontal, MoveVertical, Sparkles, Trash2 } from '@lucide/vue';
import { useCanvasController } from './useCanvasController';

const controller = useCanvasController();
</script>

<template>
  <Teleport defer to="#pipeline-toolbar">
    <div class="ptoolbar" role="toolbar" aria-label="Pipeline canvas tools">
      <div class="ptoolbar-group" role="group" aria-label="Layout direction">
        <button
          type="button"
          class="ptoolbar-btn"
          :class="{ active: controller.direction.value === 'LR' }"
          aria-label="Left-to-right layout"
          title="Left → right"
          @click="controller.setDirection('LR')"
        >
          <MoveHorizontal :size="15" :stroke-width="2" />
        </button>
        <button
          type="button"
          class="ptoolbar-btn"
          :class="{ active: controller.direction.value === 'TB' }"
          aria-label="Top-to-bottom layout"
          title="Top ↓ bottom"
          @click="controller.setDirection('TB')"
        >
          <MoveVertical :size="15" :stroke-width="2" />
        </button>
      </div>

      <span class="ptoolbar-sep" aria-hidden="true" />

      <button
        type="button"
        class="ptoolbar-btn"
        aria-label="Auto-layout"
        title="Auto-layout (ELK)"
        @click="controller.autoLayout()"
      >
        <Sparkles :size="15" :stroke-width="2" />
      </button>
      <button
        type="button"
        class="ptoolbar-btn"
        aria-label="Fit view"
        title="Fit to view"
        @click="controller.fitView()"
      >
        <Maximize2 :size="15" :stroke-width="2" />
      </button>
      <button
        type="button"
        class="ptoolbar-btn"
        :disabled="!controller.hasSelection.value"
        aria-label="Delete selected"
        title="Delete selected"
        @click="controller.deleteSelected()"
      >
        <Trash2 :size="15" :stroke-width="2" />
      </button>

      <span class="ptoolbar-sep" aria-hidden="true" />

      <button
        type="button"
        class="ptoolbar-btn"
        :class="{ active: controller.paletteOpen.value }"
        aria-label="Toggle node palette"
        title="Node palette"
        @click="controller.togglePalette()"
      >
        <LayoutGrid :size="15" :stroke-width="2" />
      </button>
    </div>
  </Teleport>
</template>

<style scoped>
.ptoolbar {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  padding: 0.3rem 0.4rem;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: color-mix(in srgb, var(--bg-main) 90%, var(--bg-sidebar));
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

.ptoolbar-group {
  display: inline-flex;
  gap: 0.15rem;
}

.ptoolbar-sep {
  width: 1px;
  height: 18px;
  margin: 0 0.2rem;
  background: var(--border-color);
}

.ptoolbar-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0.32rem;
  border: 1px solid transparent;
  border-radius: 6px;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  transition: background 0.12s ease, color 0.12s ease, border-color 0.12s ease;
}

.ptoolbar-btn:hover:not(:disabled) {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.ptoolbar-btn.active {
  color: var(--text-accent);
  background: color-mix(in srgb, var(--text-accent) 12%, transparent);
  border-color: color-mix(in srgb, var(--text-accent) 35%, transparent);
}

.ptoolbar-btn:disabled {
  cursor: default;
  opacity: 0.45;
}
</style>
