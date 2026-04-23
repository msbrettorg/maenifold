<!-- audit-ok: max-loc -->
<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { ChevronRight, ArrowUp, ArrowDown, Brain, X } from 'lucide-vue-next'
import { Button } from '@/components/ui/button'
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
      :class="['step-card w-full min-w-0 flex flex-col gap-2 p-3 bg-[var(--color-bg-elevated)] border border-[var(--color-border-subtle)] border-l-4 border-l-transparent rounded-[var(--radius-md)] cursor-pointer text-left transition-[background,border-color,box-shadow] duration-[var(--transition-fast)]', { 'step-card--selected': selected }]"
    role="button"
    tabindex="0"
    @click="selectStep"
    @keydown="handleKeydown"
    >
    <div v-if="confirmingRemove" class="flex items-center gap-3 [font-weight:var(--weight-body)] text-[length:var(--text-caption)] leading-[1.4] [font-family:var(--font-body)] text-[var(--color-text-secondary)]">
      <div class="flex-1 min-w-0 flex flex-col gap-1">
        <span class="overflow-hidden text-ellipsis">Remove "{{ step.name }}"?</span>
        <span v-if="referenceWarning" class="overflow-hidden text-ellipsis text-[var(--color-status-abandoned)] text-[length:var(--text-tiny)]">{{ referenceWarning }}</span>
      </div>
      <Button variant="outline" size="sm" class="min-h-7 px-2 border border-[var(--color-border-subtle)] rounded-[var(--radius-sm)] bg-transparent text-[var(--color-text-primary)] [font:inherit] cursor-pointer hover:bg-[var(--color-bg-surface)]" @click.stop="confirmingRemove = false">Cancel</Button>
      <Button variant="destructive" size="sm" class="card-confirm-btn--danger min-h-7 px-2 border border-[var(--color-status-abandoned)] rounded-[var(--radius-sm)] bg-transparent text-[var(--color-status-abandoned)] [font:inherit] cursor-pointer hover:bg-[var(--color-bg-surface)]" @click.stop="emit('remove', step.id)">Remove</Button>
    </div>
    <template v-else>
      <div class="flex items-center justify-between gap-2">
        <Button
          variant="ghost"
          size="icon-xs"
          class="card-disclosure w-[18px] h-[18px] p-0 border-0 rounded-[var(--radius-sm)] bg-transparent text-[var(--color-text-secondary)] cursor-pointer inline-flex items-center justify-center shrink-0 transition-[color,background] duration-[var(--transition-fast)]"
          :aria-label="expanded ? `Collapse ${step.name}` : `Expand ${step.name}`"
          :aria-expanded="expanded"
          @click.stop="emit('toggle-expanded', step.id)"
        >
          <ChevronRight class="size-3.5 transition-transform" :class="{ 'rotate-90': expanded }" :stroke-width="1.5" />
        </Button>
        <span class="min-w-0 [font-weight:var(--weight-title)] text-[length:var(--text-caption)] leading-[1.4] [font-family:var(--font-body)] text-[var(--color-text-primary)] whitespace-nowrap overflow-hidden text-ellipsis">{{ cleanName }}</span>
        <div class="flex items-center gap-2">
          <Button
            variant="ghost"
            size="icon-xs"
            class="card-move-btn w-5 h-5 p-0 border border-[var(--color-border-subtle)] rounded-[var(--radius-sm)] bg-transparent text-[var(--color-text-secondary)] cursor-pointer [font-weight:var(--weight-title)] text-[length:var(--text-2xs)] leading-none [font-family:var(--font-code)] disabled:opacity-35 disabled:cursor-default"
            :disabled="!canMoveUp"
            aria-label="Move step up"
            @click.stop="emit('move-up', step.id)"
          >
            <ArrowUp class="size-3.5" :stroke-width="1.5" />
          </Button>
          <Button
            variant="ghost"
            size="icon-xs"
            class="card-move-btn w-5 h-5 p-0 border border-[var(--color-border-subtle)] rounded-[var(--radius-sm)] bg-transparent text-[var(--color-text-secondary)] cursor-pointer [font-weight:var(--weight-title)] text-[length:var(--text-2xs)] leading-none [font-family:var(--font-code)] disabled:opacity-35 disabled:cursor-default"
            :disabled="!canMoveDown"
            aria-label="Move step down"
            @click.stop="emit('move-down', step.id)"
          >
            <ArrowDown class="size-3.5" :stroke-width="1.5" />
          </Button>
          <span
            v-if="step.requiresSequentialThinking"
            class="shrink-0 text-[length:var(--text-body-sm)] text-[var(--color-node-thinking)]"
            title="Requires sequential thinking"
          ><Brain class="size-3.5" :stroke-width="1.5" /></span>
          <Button
            v-if="canRemove"
            variant="ghost"
            size="icon-xs"
            class="card-remove-btn w-5 h-5 p-0 border-none rounded-[var(--radius-sm)] bg-transparent text-[var(--color-text-secondary)] cursor-pointer opacity-0 transition-[opacity,background,color] duration-[var(--transition-fast)]"
            :aria-label="`Remove ${step.name}`"
            @click.stop="confirmingRemove = true"
          >
            <X class="size-3.5" :stroke-width="1.5" />
          </Button>
        </div>
      </div>
      <p v-if="step.description" class="card-desc m-0 [font-weight:var(--weight-body)] text-[length:var(--text-caption)] leading-[1.5] [font-family:var(--font-body)] text-[var(--color-text-secondary)] [-webkit-box-orient:vertical] overflow-hidden" :class="{ 'card-desc--expanded': expanded }">{{ step.description }}</p>
    </template>
  </div>
</template>

<!-- audit-ok: dynamic-binding (hover/focus state color-mix, parent selector visibility) -->
<style scoped>
.step-card:hover {
  background: var(--color-bg-surface);
  box-shadow: var(--shadow-8);
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
  box-shadow: 0 0 0 1px color-mix(in srgb, var(--color-node-workflow) 28%, transparent), 0 0 18px color-mix(in srgb, var(--color-node-workflow) 10%, transparent);
}

.card-disclosure:hover {
  background: color-mix(in srgb, var(--color-node-workflow) 12%, transparent);
  color: var(--color-node-workflow);
}

.card-move-btn:hover:not(:disabled) {
  background: var(--color-bg-surface);
  color: var(--color-node-workflow);
}

.card-desc {
  display: -webkit-box;
  -webkit-line-clamp: 2;
}

.card-desc--expanded {
  display: block;
  -webkit-line-clamp: unset;
  overflow: visible;
  white-space: pre-wrap;
}

.step-card:hover .card-remove-btn,
.step-card:focus-within .card-remove-btn {
  opacity: 1;
}

.card-remove-btn:hover {
  background: color-mix(in srgb, var(--color-status-abandoned) 15%, transparent);
  color: var(--color-status-abandoned);
}
</style>
