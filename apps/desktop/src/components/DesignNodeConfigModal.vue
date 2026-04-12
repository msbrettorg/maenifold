<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue';
import DesignNodeConfigPanel from './DesignNodeConfigPanel.vue';

defineProps<{
  selectedNodeId: string | null;
  selectedEdge: { source: string; target: string } | null;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
}>();

function handleKeyDown(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    emit('close');
  }
}

onMounted(() => document.addEventListener('keydown', handleKeyDown));
onUnmounted(() => document.removeEventListener('keydown', handleKeyDown));
</script>

<template>
  <Teleport to="body">
    <div class="config-overlay" @click.self="emit('close')">
      <div class="config-panel">
        <header class="config-header">
          <span class="config-title">Node Configuration</span>
          <button
            class="config-close"
            type="button"
            aria-label="Close configuration"
            title="Close (Esc)"
            @click="emit('close')"
          >
            ×
          </button>
        </header>
        <div class="config-body">
          <DesignNodeConfigPanel
            :selected-node-id="selectedNodeId"
            :selected-edge="selectedEdge"
          />
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.config-overlay {
  position: fixed;
  inset: 0;
  z-index: 1000;
  display: grid;
  place-items: center;
  background: rgba(0, 0, 0, 0.6);
}

.config-panel {
  width: min(560px, 90vw);
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  background: var(--surface-container, #201f1f);
  border-radius: var(--radius-xl);
  overflow: hidden;
  border: 1px solid var(--border-default);
  box-shadow: 0 20px 48px rgba(0, 0, 0, 0.35);
}

.config-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-3) var(--space-4);
  background: var(--surface-container-highest, #353534);
}

.config-title {
  font-family: var(--font-headline, var(--font-sans));
  font-size: var(--text-sm);
  font-weight: 600;
  color: var(--text-primary);
}

.config-close {
  background: none;
  border: none;
  color: var(--text-muted);
  font-size: var(--text-xl);
  cursor: pointer;
  padding: var(--space-1);
  line-height: 1;
  border-radius: var(--radius-sm);
}

.config-close:hover {
  color: var(--text-primary);
  background: var(--bg-surface);
}

.config-close:focus-visible {
  outline: 2px solid var(--border-focus);
  outline-offset: 2px;
}

.config-body {
  flex: 1;
  overflow-y: auto;
}

.config-body :deep(.inspector) {
  border-left: none;
  height: auto;
}
</style>
