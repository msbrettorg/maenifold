<script setup lang="ts">
import { onBeforeUnmount, ref } from 'vue'

const props = defineProps<{
  dragging: boolean
  side: 'left' | 'right'
}>()

const emit = defineEmits<{
  (event: 'drag-start'): void
  (event: 'resize', delta: number): void
  (event: 'drag-end'): void
  (event: 'reset'): void
}>()

const KEYBOARD_STEP = 10
const KEYBOARD_STEP_LARGE = 50

const handleRef = ref<HTMLElement | null>(null)
const activePointerId = ref<number | null>(null)
const lastX = ref(0)

function handlePointerDown(event: PointerEvent) {
  if (!event.isPrimary || activePointerId.value !== null) return

  event.preventDefault()
  activePointerId.value = event.pointerId
  lastX.value = event.clientX
  handleRef.value?.setPointerCapture(event.pointerId)
  emit('drag-start')
}

function handlePointerMove(event: PointerEvent) {
  if (event.pointerId !== activePointerId.value) return

  const delta = event.clientX - lastX.value
  if (delta === 0) return

  lastX.value = event.clientX
  emit('resize', delta)
}

function finishDrag(event?: PointerEvent) {
  if (activePointerId.value === null) return
  if (event && event.pointerId !== activePointerId.value) return

  const pointerId = activePointerId.value
  activePointerId.value = null

  if (handleRef.value?.hasPointerCapture(pointerId)) {
    handleRef.value.releasePointerCapture(pointerId)
  }

  emit('drag-end')
}

function handleKeyDown(event: KeyboardEvent) {
  if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
    event.preventDefault()
    const magnitude = event.shiftKey ? KEYBOARD_STEP_LARGE : KEYBOARD_STEP
    const delta = event.key === 'ArrowRight' ? magnitude : -magnitude
    emit('resize', delta)
    emit('drag-end')
  } else if (event.key === 'Home') {
    event.preventDefault()
    emit('reset')
  }
}

function handleDoubleClick(event: MouseEvent) {
  event.preventDefault()
  emit('reset')
}

onBeforeUnmount(() => {
  finishDrag()
})
</script>

<template>
  <div
    ref="handleRef"
    class="panel-resize-handle"
    :class="{ 'panel-resize-handle--dragging': props.dragging }"
    :data-side="props.side"
    role="separator"
    aria-orientation="vertical"
    :aria-label="props.side === 'left' ? 'Resize workflow browser' : 'Resize workflow inspector'"
    tabindex="0"
    @pointerdown="handlePointerDown"
    @pointermove="handlePointerMove"
    @pointerup="finishDrag"
    @pointercancel="finishDrag"
    @lostpointercapture="finishDrag"
    @keydown="handleKeyDown"
    @dblclick="handleDoubleClick"
    @dragstart.prevent
  >
    <span class="panel-resize-handle__grip" aria-hidden="true" />
  </div>
</template>

<style scoped>
.panel-resize-handle {
  width: 12px;
  flex: 0 0 auto;
  align-self: stretch;
  cursor: col-resize;
  position: relative;
  touch-action: none;
  background: transparent;
}

.panel-resize-handle:hover,
.panel-resize-handle--dragging {
  background: transparent;
}

.panel-resize-handle:focus {
  outline: none;
}

.panel-resize-handle:focus-visible {
  outline: 2px solid var(--color-brand, var(--ring, #6366f1));
  outline-offset: -2px;
  background: color-mix(in srgb, var(--color-brand, var(--ring, #6366f1)) 12%, transparent);
}

.panel-resize-handle__grip {
  display: none;
}
</style>
