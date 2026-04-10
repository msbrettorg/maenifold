<script setup lang="ts">
import { ref, watch } from 'vue'
import type { WorkflowStepDefinition } from '../lib/workflow-asset-types'

const props = defineProps<{
  canRemove: boolean
  canMoveDown: boolean
  canMoveUp: boolean
  referenceWarning: string | null
  selected: boolean
  step: WorkflowStepDefinition
}>()

const emit = defineEmits<{
  'move-down': [stepId: string]
  'move-up': [stepId: string]
  remove: [stepId: string]
  select: [stepId: string]
}>()

const confirmingRemove = ref(false)

watch(
  () => props.canRemove,
  (canRemove) => {
    if (!canRemove) {
      confirmingRemove.value = false
    }
  },
)

watch(
  () => props.step.id,
  () => {
    confirmingRemove.value = false
  },
)

function selectStep() {
  if (confirmingRemove.value) return
  emit('select', props.step.id)
}

function handleKeydown(event: KeyboardEvent) {
  if (event.key !== 'Enter' && event.key !== ' ') return
  event.preventDefault()
  selectStep()
}
</script>

<template>
    <div
      :class="['step-card', { 'step-card--selected': selected }]"
    role="button"
    tabindex="0"
    @click="selectStep"
    @keydown="handleKeydown"
    >
    <div v-if="confirmingRemove" class="card-confirm-row">
      <div class="confirm-copy">
        <span class="confirm-label">Remove "{{ step.name }}"?</span>
        <span v-if="referenceWarning" class="confirm-warning">{{ referenceWarning }}</span>
      </div>
      <button type="button" class="card-confirm-btn" @click.stop="confirmingRemove = false">Cancel</button>
      <button type="button" class="card-confirm-btn card-confirm-btn--danger" @click.stop="emit('remove', step.id)">Remove</button>
    </div>
    <template v-else>
      <div class="card-top">
        <span class="card-name">{{ step.name }}</span>
        <div class="card-actions">
          <button
            class="card-move-btn"
            :disabled="!canMoveUp"
            type="button"
            aria-label="Move step up"
            @click.stop="emit('move-up', step.id)"
          >
            ↑
          </button>
          <button
            class="card-move-btn"
            :disabled="!canMoveDown"
            type="button"
            aria-label="Move step down"
            @click.stop="emit('move-down', step.id)"
          >
            ↓
          </button>
          <span
            v-if="step.requiresSequentialThinking"
            class="thinking-icon"
            title="Requires sequential thinking"
          >🧠</span>
          <button
            v-if="canRemove"
            type="button"
            class="card-remove-btn"
            :aria-label="`Remove ${step.name}`"
            @click.stop="confirmingRemove = true"
          >
            ×
          </button>
        </div>
      </div>
      <p class="card-desc">{{ step.description }}</p>
    </template>
  </div>
</template>

<style scoped>
.step-card {
  width: 100%;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  padding: var(--space-3) var(--space-4);
  background: var(--surface-container);
  border: 1px solid var(--border-default);
  border-left: 4px solid transparent;
  border-radius: var(--radius-md);
  cursor: pointer;
  text-align: left;
  transition: background var(--transition-fast), border-color var(--transition-fast), box-shadow var(--transition-fast);
}

.step-card:hover {
  background: var(--bg-surface);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
}

.step-card:focus-visible {
  outline: 2px solid color-mix(in srgb, var(--color-node-workflow) 50%, transparent);
  outline-offset: 2px;
}

.step-card--selected {
  background: var(--bg-surface);
  border-left-color: var(--color-node-workflow);
  border-color: color-mix(in srgb, var(--color-node-workflow) 30%, var(--border-default));
  box-shadow: 0 0 0 1px color-mix(in srgb, var(--color-node-workflow) 20%, transparent);
}

.card-actions {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.card-move-btn {
  width: 20px;
  height: 20px;
  padding: 0;
  border: 1px solid var(--border-default);
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  font: 600 10px/1 var(--font-mono);
}

.card-move-btn:hover:not(:disabled) {
  background: var(--bg-surface);
  color: var(--color-node-workflow);
}

.card-move-btn:disabled {
  opacity: 0.35;
  cursor: default;
}

.card-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
}

.card-name {
  min-width: 0;
  font: 600 var(--text-sm)/1.4 var(--font-sans);
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.thinking-icon {
  flex-shrink: 0;
  font-size: var(--text-base);
  color: var(--color-node-thinking);
}

.card-desc {
  margin: 0;
  font: 400 var(--text-sm)/1.5 var(--font-sans);
  color: var(--text-secondary);
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.card-remove-btn {
  width: 20px;
  height: 20px;
  padding: 0;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  opacity: 0;
  transition: opacity var(--transition-fast), background var(--transition-fast), color var(--transition-fast);
}

.step-card:hover .card-remove-btn,
.step-card:focus-within .card-remove-btn {
  opacity: 1;
}

.card-remove-btn:hover {
  background: color-mix(in srgb, var(--color-status-abandoned) 15%, transparent);
  color: var(--color-status-abandoned);
}

.card-confirm-row {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  font: 400 var(--text-sm)/1.4 var(--font-sans);
  color: var(--text-secondary);
}

.confirm-copy {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.confirm-label,
.confirm-warning {
  overflow: hidden;
  text-overflow: ellipsis;
}

.confirm-warning {
  color: var(--color-status-abandoned);
  font-size: var(--text-xs);
}

.card-confirm-btn {
  min-height: 28px;
  padding: 0 var(--space-2);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-primary);
  font: inherit;
  cursor: pointer;
}

.card-confirm-btn:hover {
  background: var(--bg-surface);
}

.card-confirm-btn--danger {
  border-color: var(--color-status-abandoned);
  color: var(--color-status-abandoned);
}
</style>
