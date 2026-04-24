<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { ChevronRight, ArrowUp, ArrowDown, Brain, X } from 'lucide-vue-next'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from '@/components/ui/collapsible'
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
  <Card
    role="button"
    tabindex="0"
    :aria-pressed="selected"
    :class="[
      'group/step w-full min-w-0 flex flex-col gap-2 p-3 rounded-md border border-l-4 cursor-pointer text-left shadow-none ring-0 bg-elevated transition-colors',
      'hover:bg-surface focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-node-workflow/50',
      selected
        ? 'border-l-node-workflow border-border-subtle bg-surface ring-1 ring-node-workflow/30'
        : 'border-l-transparent border-border-subtle',
    ]"
    @click="selectStep"
    @keydown="handleKeydown"
  >
    <div
      v-if="confirmingRemove"
      class="flex items-center gap-3 text-xs leading-tight text-text-secondary"
    >
      <div class="flex-1 min-w-0 flex flex-col gap-1">
        <span class="overflow-hidden text-ellipsis">Remove "{{ step.name }}"?</span>
        <span
          v-if="referenceWarning"
          class="overflow-hidden text-ellipsis text-status-abandoned text-[0.6875rem]"
        >{{ referenceWarning }}</span>
      </div>
      <Button
        variant="outline"
        size="sm"
        class="min-h-7 px-2 rounded-sm bg-transparent text-text-primary hover:bg-surface"
        @click.stop="confirmingRemove = false"
      >Cancel</Button>
      <Button
        variant="outline"
        size="sm"
        class="min-h-7 px-2 rounded-sm bg-transparent border-status-abandoned text-status-abandoned hover:bg-status-abandoned/10 hover:text-status-abandoned"
        @click.stop="emit('remove', step.id)"
      >Remove</Button>
    </div>

    <Collapsible
      v-else
      :open="expanded"
      class="flex flex-col gap-2 min-w-0"
    >
      <div class="flex items-center justify-between gap-2 min-w-0">
        <CollapsibleTrigger as-child>
          <Button
            variant="ghost"
            size="icon"
            class="size-[18px] p-0 rounded-sm bg-transparent text-text-secondary shrink-0 hover:bg-node-workflow/10 hover:text-node-workflow"
            :aria-label="expanded ? `Collapse ${step.name}` : `Expand ${step.name}`"
            :aria-expanded="expanded"
            @click.stop="emit('toggle-expanded', step.id)"
          >
            <ChevronRight
              class="size-3.5 transition-transform"
              :class="{ 'rotate-90': expanded }"
              :stroke-width="1.5"
            />
          </Button>
        </CollapsibleTrigger>

        <span
          class="flex-1 min-w-0 font-medium text-xs leading-tight text-text-primary whitespace-nowrap overflow-hidden text-ellipsis"
        >{{ cleanName }}</span>

        <div class="flex items-center gap-2 shrink-0">
          <Button
            variant="ghost"
            size="icon"
            class="size-5 p-0 rounded-sm border border-border-subtle bg-transparent text-text-secondary hover:bg-surface hover:text-node-workflow disabled:opacity-35 disabled:cursor-default"
            :disabled="!canMoveUp"
            aria-label="Move step up"
            @click.stop="emit('move-up', step.id)"
          >
            <ArrowUp class="size-3.5" :stroke-width="1.5" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            class="size-5 p-0 rounded-sm border border-border-subtle bg-transparent text-text-secondary hover:bg-surface hover:text-node-workflow disabled:opacity-35 disabled:cursor-default"
            :disabled="!canMoveDown"
            aria-label="Move step down"
            @click.stop="emit('move-down', step.id)"
          >
            <ArrowDown class="size-3.5" :stroke-width="1.5" />
          </Button>
          <Badge
            v-if="step.requiresSequentialThinking"
            variant="outline"
            class="h-5 gap-1 px-1.5 border-node-thinking/40 text-node-thinking bg-transparent"
            title="Requires sequential thinking"
          >
            <Brain class="size-3.5" :stroke-width="1.5" />
          </Badge>
          <Button
            v-if="canRemove"
            variant="ghost"
            size="icon"
            class="size-5 p-0 rounded-sm bg-transparent text-text-secondary opacity-0 transition-opacity group-hover/step:opacity-100 group-focus-within/step:opacity-100 hover:bg-status-abandoned/15 hover:text-status-abandoned"
            :aria-label="`Remove ${step.name}`"
            @click.stop="confirmingRemove = true"
          >
            <X class="size-3.5" :stroke-width="1.5" />
          </Button>
        </div>
      </div>

      <p
        v-if="step.description && !expanded"
        class="m-0 text-xs leading-snug text-text-secondary line-clamp-2"
      >{{ step.description }}</p>

      <CollapsibleContent v-if="step.description">
        <p class="m-0 text-xs leading-snug text-text-secondary whitespace-pre-wrap">{{ step.description }}</p>
      </CollapsibleContent>
    </Collapsible>
  </Card>
</template>
