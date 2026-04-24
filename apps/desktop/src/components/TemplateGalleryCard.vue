<script setup lang="ts">
import { computed } from 'vue';
import type { Component } from 'vue';
import { FlaskConical, Brain, PenTool, Globe, Briefcase } from 'lucide-vue-next';
import { Card } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import type { PipelineTemplate } from '../lib/pipeline-templates';

const props = defineProps<{
  selected: boolean;
  template: PipelineTemplate;
}>();

const emit = defineEmits<{
  select: [];
}>();

const iconMap: Record<string, Component> = {
  'flask-conical': FlaskConical,
  brain: Brain,
  'pen-tool': PenTool,
  globe: Globe,
  briefcase: Briefcase,
};

const accentBarClass: Record<string, string> = {
  'research-and-verify': 'bg-brand',
  'think-and-critique': 'bg-node-thinking',
  'draft-and-refine': 'bg-status-active',
  'explore-and-synthesize': 'bg-status-completed',
  'deal-preparation': 'bg-status-active',
};

const accentClass = computed(() => accentBarClass[props.template.id] ?? 'bg-brand');
const iconComponent = computed(() => iconMap[props.template.icon] ?? FlaskConical);

function onActivate() {
  emit('select');
}

function onKey(event: KeyboardEvent) {
  if (event.key === 'Enter' || event.key === ' ') {
    event.preventDefault();
    emit('select');
  }
}
</script>

<template>
  <Card
    role="button"
    tabindex="0"
    :aria-pressed="props.selected"
    class="relative flex min-h-36 cursor-pointer flex-col items-start gap-1.5 overflow-hidden py-3.5 pl-4 pr-3.5 text-left transition-colors duration-150 hover:bg-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
    :class="props.selected ? 'bg-accent ring-2 ring-ring' : ''"
    @click="onActivate"
    @keydown="onKey"
  >
    <span class="absolute inset-y-0 left-0 w-1" :class="accentClass" aria-hidden="true" />
    <component :is="iconComponent" class="size-5 text-muted-foreground" :stroke-width="1.5" />
    <span class="text-sm font-semibold text-foreground">{{ props.template.label }}</span>
    <span class="text-xs leading-snug text-muted-foreground">{{ props.template.description }}</span>
    <Badge
      v-if="props.template.category === 'enterprise'"
      variant="secondary"
      class="mt-auto"
    >Enterprise</Badge>
  </Card>
</template>
