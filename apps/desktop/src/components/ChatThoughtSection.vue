<script setup lang="ts">
import { computed } from 'vue';
import { ChevronDown, ChevronUp } from 'lucide-vue-next';
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from '@/components/ui/collapsible';
import { Badge } from '@/components/ui/badge';
import MarkdownView from './MarkdownView.vue';

const props = defineProps<{
  messageId: string;
  thought: string;
  expanded: boolean;
}>();

const emit = defineEmits<{ toggle: [messageId: string] }>();

const contentId = computed(() => `thought-content-${props.messageId}`);

function handleUpdate(open: boolean) {
  if (open !== props.expanded) {
    emit('toggle', props.messageId);
  }
}
</script>

<template>
  <Collapsible
    :open="expanded"
    class="mb-3 overflow-hidden rounded border border-border-subtle"
    role="region"
    aria-label="Thinking"
    @update:open="handleUpdate"
  >
    <CollapsibleTrigger
      as-child
      :aria-controls="contentId"
    >
      <Badge
        variant="secondary"
        class="flex w-full cursor-pointer items-center gap-2 rounded-none border-none bg-surface-high px-3 py-2 text-left text-xs font-medium text-text-secondary transition-colors duration-150 hover:bg-surface-highest"
      >
        <span class="shrink-0 text-base">💭</span>
        <span class="flex-1 text-xs">{{ expanded ? 'Hide thinking' : 'Show thinking' }}</span>
        <ChevronUp v-if="expanded" class="h-3 w-3 text-text-secondary" />
        <ChevronDown v-else class="h-3 w-3 text-text-secondary" />
      </Badge>
    </CollapsibleTrigger>
    <CollapsibleContent
      :id="contentId"
      class="bg-surface-container py-3 pl-5 pr-4 text-base italic leading-relaxed text-text-secondary"
    >
      <MarkdownView :content="thought" class="text-text-secondary italic" />
    </CollapsibleContent>
  </Collapsible>
</template>
