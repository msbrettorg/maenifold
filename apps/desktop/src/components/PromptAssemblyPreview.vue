<script setup lang="ts">
import { computed, ref } from 'vue';
import { ChevronRight } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import MarkdownView from './MarkdownView.vue';

interface PreviewSection {
  key: string;
  label: string;
  content: string;
}

const props = defineProps<{
  assembledPrompt: string;
}>();

const isExpanded = ref(false);

const sections = computed<PreviewSection[]>(() => {
  const trimmedPrompt = props.assembledPrompt.trim();
  if (!trimmedPrompt) {
    return [];
  }

  const chunks = trimmedPrompt.split(/\n{2,}/).filter((chunk) => chunk.trim().length > 0);
  if (chunks.length < 2) {
    return [{ key: 'full', label: 'Full Prompt', content: trimmedPrompt }];
  }

  const predecessorChunks: string[] = [];
  let index = 0;

  while (index < chunks.length - 2 && chunks[index].startsWith('### From ')) {
    predecessorChunks.push(chunks[index], chunks[index + 1]);
    index += 2;
  }

  const remainder = chunks.slice(index);
  const parsedSections: PreviewSection[] = [];

  if (predecessorChunks.length > 0) {
    parsedSections.push({
      key: 'predecessor',
      label: 'Predecessor Context',
      content: predecessorChunks.join('\n\n'),
    });
  }

  if (remainder.length > 1) {
    parsedSections.push({
      key: 'ambient',
      label: 'Ambient Prompt',
      content: remainder.slice(0, -1).join('\n\n'),
    });
    parsedSections.push({
      key: 'instruction',
      label: 'Node Instruction',
      content: remainder[remainder.length - 1],
    });
  } else if (remainder.length === 1) {
    parsedSections.push({
      key: predecessorChunks.length > 0 ? 'instruction' : 'full',
      label: predecessorChunks.length > 0 ? 'Node Instruction' : 'Full Prompt',
      content: remainder[0],
    });
  }

  return parsedSections.filter((section) => section.content.trim().length > 0);
});
</script>

<template>
  <Card class="bg-surface border-border-subtle">
    <CardContent class="p-0">
      <Button
        variant="ghost"
        class="w-full flex items-center justify-between gap-3 px-3.5 py-3 h-auto rounded-md text-text-primary font-semibold hover:bg-surface-hover"
        :aria-expanded="isExpanded"
        @click="isExpanded = !isExpanded"
      >
        <span>Assembled Prompt</span>
        <ChevronRight
          class="h-4 w-4 text-text-secondary transition-transform"
          :class="isExpanded ? 'rotate-90' : ''"
          aria-hidden="true"
        />
      </Button>

      <div v-if="isExpanded" class="flex flex-col gap-3 px-3.5 pb-3.5">
        <div
          v-for="section in sections"
          :key="section.key"
          class="flex flex-col gap-1.5"
        >
          <div class="text-xs font-semibold tracking-wide text-text-secondary">
            {{ section.label }}
          </div>
          <ScrollArea class="max-h-48 rounded-md border border-border-subtle bg-bg-app">
            <MarkdownView
              :content="section.content"
              class="p-3 text-text-primary font-mono text-sm leading-normal [overflow-wrap:anywhere] [&_*]:mt-0 [&_p:last-child]:mb-0 [&_code]:font-mono [&_code]:text-xs [&_code]:bg-bg-app [&_code]:rounded [&_code]:px-1 [&_code]:py-0.5 [&_pre]:mt-3 [&_pre]:mb-0 [&_pre]:p-3 [&_pre]:bg-bg-app [&_pre]:rounded [&_pre]:overflow-x-auto"
            />
          </ScrollArea>
        </div>
      </div>
    </CardContent>
  </Card>
</template>
