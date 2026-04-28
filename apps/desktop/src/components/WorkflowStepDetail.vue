<script setup lang="ts">
import { computed } from 'vue';
import { Brain, ChevronDown, ChevronUp, CircleDot, Circle, CheckCircle2 } from 'lucide-vue-next';
import { Badge } from '@/components/ui/badge';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from '@/components/ui/collapsible';
import type { WorkflowStep } from '../lib/workflow-types';
import MarkdownView from './MarkdownView.vue';

const props = defineProps<{
  step: WorkflowStep;
  expanded: boolean;
}>();

const stepTitleId = `step-title-${Math.random().toString(36).slice(2, 10)}`;

const emit = defineEmits<{
  toggle: [];
}>();

const statusIconClass = computed(() => {
  switch (props.step.status) {
    case 'completed': return 'text-status-completed';
    case 'active':    return 'text-status-active';
    case 'pending':   return 'text-text-tertiary';
    default:          return 'text-text-tertiary';
  }
});

function relativeTime(iso: string | undefined): string {
  if (!iso) return '';
  const diff = Date.now() - new Date(iso).getTime();
  if (diff < 0) return 'just now';
  const s = Math.floor(diff / 1000);
  if (s < 60) return `${s}s ago`;
  const m = Math.floor(s / 60);
  if (m < 60) return `${m} min ago`;
  const h = Math.floor(m / 60);
  if (h < 24) return `${h}h ago`;
  return `${Math.floor(h / 24)}d ago`;
}

function handleToggle(event: Event) {
  if (!props.step.response) return;
  event.preventDefault();
  emit('toggle');
}
</script>

<template>
  <li
    class="flex flex-col rounded overflow-hidden"
    role="article"
    :aria-labelledby="stepTitleId"
  >
    <Collapsible :open="expanded" class="flex flex-col">
      <CollapsibleTrigger
        as-child
        :disabled="!step.response"
      >
        <div
          class="flex items-center gap-1.5 h-7 px-1 rounded select-none"
          :class="step.response ? 'cursor-pointer hover:bg-surface' : 'cursor-default'"
          @click="handleToggle"
        >
          <span class="font-mono text-[0.6875rem] min-w-6 text-right text-text-tertiary">
            {{ String(step.stepNumber).padStart(2, '0') }}
          </span>

          <span class="text-xs shrink-0 inline-flex items-center justify-center" :class="statusIconClass">
            <CheckCircle2 v-if="step.status === 'completed'" class="size-3.5" :stroke-width="2" />
            <CircleDot v-else-if="step.status === 'active'" class="size-3.5 animate-pulse" :stroke-width="2" />
            <Circle v-else class="size-3.5" :stroke-width="2" />
          </span>

          <span
            :id="stepTitleId"
            class="flex-1 whitespace-nowrap overflow-hidden text-ellipsis text-xs text-text-primary"
          >{{ step.name }}</span>

          <Badge
            v-if="step.requiresSequentialThinking"
            variant="outline"
            class="h-5 gap-1 px-1.5 border-node-thinking/40 text-node-thinking bg-transparent shrink-0"
            title="Sequential thinking"
          >
            <Brain class="size-3.5" :stroke-width="1.5" />
          </Badge>

          <span
            v-if="step.respondedAt"
            class="text-[0.6875rem] shrink-0 whitespace-nowrap text-text-secondary"
          >{{ relativeTime(step.respondedAt) }}</span>

          <span
            v-if="step.response"
            class="text-[0.6875rem] shrink-0 whitespace-nowrap text-text-secondary inline-flex items-center"
          >
            <ChevronUp v-if="expanded" class="size-3.5" :stroke-width="1.5" />
            <ChevronDown v-else class="size-3.5" :stroke-width="1.5" />
          </span>
        </div>
      </CollapsibleTrigger>

      <div
        v-if="step.requiresSequentialThinking && step.submachineSessionId"
        class="flex items-center gap-1 pl-9 mb-0.5"
      >
        <span class="text-status-active text-[0.6875rem] leading-none">│</span>
        <span class="font-mono text-[0.6875rem] whitespace-nowrap overflow-hidden text-ellipsis text-text-secondary">
          → thinking: {{ step.submachineSessionId }}
        </span>
      </div>

      <CollapsibleContent v-if="step.response">
        <ScrollArea class="mt-1 mb-1.5 ml-9 max-h-[400px] rounded bg-surface">
          <MarkdownView
            :content="step.response"
            class="py-2 px-3 text-xs leading-relaxed text-text-primary
                   [&_p]:my-1 [&_p:first-child]:mt-0 [&_p:last-child]:mb-0
                   [&_code]:font-mono [&_code]:text-[0.6875rem] [&_code]:bg-bg-elevated [&_code]:px-1 [&_code]:py-px [&_code]:rounded [&_code]:text-brand
                   [&_pre]:bg-bg-elevated [&_pre]:rounded [&_pre]:p-2
                   [&_pre_code]:bg-transparent [&_pre_code]:p-0
                   [&_table]:border-collapse [&_table]:w-full [&_table]:my-3 [&_table]:text-xs
                   [&_th]:border [&_th]:border-border-subtle [&_th]:px-3 [&_th]:py-2 [&_th]:text-left [&_th]:bg-surface [&_th]:font-semibold
                   [&_td]:border [&_td]:border-border-subtle [&_td]:px-3 [&_td]:py-2 [&_td]:text-left
                    [&_tr:nth-child(even)]:bg-bg-elevated
                    [&_ul]:my-1 [&_ul]:pl-5 [&_ol]:my-1 [&_ol]:pl-5
                    [&_li]:my-0.5"
          />
        </ScrollArea>
      </CollapsibleContent>
    </Collapsible>
  </li>
</template>
