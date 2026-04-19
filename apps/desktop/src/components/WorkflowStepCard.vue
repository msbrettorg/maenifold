<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { ChevronRight, ArrowUp, ArrowDown, Brain, X } from 'lucide-vue-next'
import type { WorkflowStepDefinition } from '../lib/workflow-asset-types'

const emojiPattern = /[\p{Emoji_Presentation}\p{Extended_Pictographic}]\uFE0F?\s*/gu

const props = defineProps<{
  canRemove: boolean
  canMoveDown: boolean
  canMoveUp: boolean
  expanded: boolean
  referenceWarning: string | null
  selected: boolean
  step: WorkflowStepDefinition
}>()

const cleanName = computed(() => props.step.name.replace(emojiPattern, '').trim())

const emit = defineEmits<{
  'move-down': [stepId: string]
  'move-up': [stepId: string]
  remove: [stepId: string]
  select: [stepId: string]
  'toggle-expanded': [stepId: string]
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
        <button
          type="button"
          class="card-disclosure"
          :aria-label="expanded ? `Collapse ${step.name}` : `Expand ${step.name}`"
          :aria-expanded="expanded"
          @click.stop="emit('toggle-expanded', step.id)"
        >
          <ChevronRight class="size-3.5 transition-transform" :class="{ 'rotate-90': expanded }" :stroke-width="1.5" />
        </button>
        <span class="card-name">{{ cleanName }}</span>
        <div class="card-actions">
          <button
            class="card-move-btn"
            :disabled="!canMoveUp"
            type="button"
            aria-label="Move step up"
            @click.stop="emit('move-up', step.id)"
          >
            <ArrowUp class="size-3.5" :stroke-width="1.5" />
          </button>
          <button
            class="card-move-btn"
            :disabled="!canMoveDown"
            type="button"
            aria-label="Move step down"
            @click.stop="emit('move-down', step.id)"
          >
            <ArrowDown class="size-3.5" :stroke-width="1.5" />
          </button>
          <span
            v-if="step.requiresSequentialThinking"
            class="thinking-icon"
            title="Requires sequential thinking"
          ><Brain class="size-3.5" :stroke-width="1.5" /></span>
          <button
            v-if="canRemove"
            type="button"
            class="card-remove-btn"
            :aria-label="`Remove ${step.name}`"
            @click.stop="confirmingRemove = true"
          >
            <X class="size-3.5" :stroke-width="1.5" />
          </button>
        </div>
      </div>
      <p v-if="step.description" class="card-desc" :class="{ 'card-desc--expanded': expanded }">{{ step.description }}</p>
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
  background: var(--color-bg-elevated);
  border: 1px solid var(--color-border-subtle);
  border-left: 4px solid transparent;
  border-radius: var(--radius-md);
  cursor: pointer;
  text-align: left;
  transition: background var(--transition-fast), border-color var(--transition-fast), box-shadow var(--transition-fast);
}

.step-card:hover {
  background: var(--color-bg-surface);
  box-shadow: 0 2px 4px rgba(0,0,0,0.14), 0 0 2px rgba(0,0,0,0.12);
}

.step-card:focus-visible {
  outline: 2px solid color-mix(in srgb, var(--color-node-workflow) 50%, transparent);
  outline-offset: 2px;
}

.step-card--selected {
  background: var(--color-bg-surface);
  border-left-color: var(--color-node-workflow);
  border-left-width: 4px;
  border-color: color-mix(in srgb, var(--color-node-workflow) 36%, var(--color-border-subtle));
  box-shadow: 0 0 0 1px color-mix(in srgb, var(--color-node-workflow) 28%, transparent),
              0 0 18px color-mix(in srgb, var(--color-node-workflow) 10%, transparent);
}

.card-disclosure {
  width: 18px;
  height: 18px;
  padding: 0;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--color-text-secondary);
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  transition: color var(--transition-fast), background var(--transition-fast);
}

.card-disclosure:hover {
  background: color-mix(in srgb, var(--color-node-workflow) 12%, transparent);
  color: var(--color-node-workflow);
}

.disclosure-caret {
  display: inline-block;
  font-size: 0.6875rem;
  line-height: 1;
  transition: transform var(--transition-fast);
}

.disclosure-caret--open {
  transform: rotate(90deg);
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
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--color-text-secondary);
  cursor: pointer;
  font: var(--weight-title) var(--text-2xs)/1 var(--font-code);
}

.card-move-btn:hover:not(:disabled) {
  background: var(--color-bg-surface);
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
  font: var(--weight-title) var(--text-caption)/1.4 var(--font-body);
  color: var(--color-text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.thinking-icon {
  flex-shrink: 0;
  font-size: var(--text-body-sm);
  color: var(--color-node-thinking);
}

.card-desc {
  margin: 0;
  font: var(--weight-body) var(--text-caption)/1.5 var(--font-body);
  color: var(--color-text-secondary);
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.card-desc--expanded {
  display: block;
  -webkit-line-clamp: unset;
  overflow: visible;
  white-space: pre-wrap;
}

.card-remove-btn {
  width: 20px;
  height: 20px;
  padding: 0;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--color-text-secondary);
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
  font: var(--weight-body) var(--text-caption)/1.4 var(--font-body);
  color: var(--color-text-secondary);
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
  font-size: var(--text-tiny);
}

.card-confirm-btn {
  min-height: 28px;
  padding: 0 var(--space-2);
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--color-text-primary);
  font: inherit;
  cursor: pointer;
}

.card-confirm-btn:hover {
  background: var(--color-bg-surface);
}

.card-confirm-btn--danger {
  border-color: var(--color-status-abandoned);
  color: var(--color-status-abandoned);
}
</style>
