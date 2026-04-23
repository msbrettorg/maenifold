<script setup lang="ts">
import { FlaskConical, Brain, PenTool, Globe, Briefcase } from 'lucide-vue-next';
import type { Component } from 'vue';
import { Button } from '@/components/ui/button';
import type { PipelineTemplate } from '../lib/pipeline-templates';

const ACCENT_BY_TEMPLATE_ID: Record<string, string> = {
  'research-and-verify': 'var(--color-brand)',
  'think-and-critique': 'var(--plum)',
  'draft-and-refine': 'var(--amber)',
  'explore-and-synthesize': 'var(--sage)',
  'deal-preparation': 'var(--amber)',
};

const props = defineProps<{
  selected: boolean;
  template: PipelineTemplate;
}>();

const iconMap: Record<string, Component> = {
  'flask-conical': FlaskConical,
  'brain': Brain,
  'pen-tool': PenTool,
  'globe': Globe,
  'briefcase': Briefcase,
};

defineEmits<{
  select: [];
}>();
</script>

<template>
  <!-- audit-ok: dynamic-binding (CSS custom property; accent looked up by template id) -->
  <Button
    type="button"
    variant="ghost"
    class="shadow-[var(--shadow-8)] border-[1.5px] border-border-card flex h-auto flex-col items-start whitespace-normal rounded-[0.25rem] font-normal text-left relative gap-[0.35rem] min-h-[9rem] py-[0.875rem] pr-[0.875rem] pl-4 cursor-pointer transition-[background,transform] duration-150 ease-in-out before:content-[''] before:absolute before:top-0 before:bottom-0 before:left-0 before:w-1 before:rounded-l-[0.25rem] before:bg-[var(--template-accent)] bg-[var(--surface-container)] hover:bg-[var(--surface-high)]"
    :class="{ 'bg-[var(--surface-high)]': props.selected }"
    :style="{ '--template-accent': ACCENT_BY_TEMPLATE_ID[props.template.id] ?? 'var(--color-brand)' }"
    @click="$emit('select')"
  >
    <component :is="iconMap[props.template.icon] ?? FlaskConical" class="size-5 text-text-secondary" :stroke-width="1.5" />
    <span class="text-[0.875rem] font-[number:var(--weight-title)] text-text-primary">{{ props.template.label }}</span>
    <span class="text-[0.8125rem] text-[var(--text-secondary)] leading-[1.3]">{{ props.template.description }}</span>
    <!-- audit-ok: dynamic-binding (color-mix on dynamic --template-accent CSS var) -->
    <span
      v-if="props.template.category === 'enterprise'"
      class="mt-auto inline-flex py-[0.125rem] px-2 rounded-[4px] text-[0.75rem] font-medium text-text-primary"
      style="background: color-mix(in srgb, var(--template-accent) 18%, transparent);"
    >Enterprise</span>
  </Button>
</template>
