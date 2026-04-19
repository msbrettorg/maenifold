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
  <Button
    type="button"
    variant="ghost"
    :class="['gallery-card shadow-[var(--shadow-8)] border-[1.5px] border-border-card flex h-auto flex-col items-start whitespace-normal rounded-[0.25rem] font-normal text-left', { 'gallery-card--selected': props.selected }]"
    :style="{ '--template-accent': ACCENT_BY_TEMPLATE_ID[props.template.id] ?? 'var(--color-brand)' }"
    @click="$emit('select')"
  >
    <component :is="iconMap[props.template.icon] ?? FlaskConical" class="gallery-card-icon size-5 text-text-secondary" :stroke-width="1.5" />
    <span class="gallery-card-label">{{ props.template.label }}</span>
    <span class="gallery-card-desc">{{ props.template.description }}</span>
    <span v-if="props.template.category === 'enterprise'" class="gallery-card-badge">Enterprise</span>
  </Button>
</template>

<style scoped>
.gallery-card {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.35rem;
  position: relative;
  min-height: 9rem;
  padding: 0.875rem 0.875rem 0.875rem 1rem;
  border-radius: 0.25rem;
  background: var(--surface-container);
  cursor: pointer;
  text-align: left;
  transition: background 0.15s ease, transform 0.15s ease;
}

.gallery-card::before {
  content: '';
  position: absolute;
  top: 0;
  bottom: 0;
  left: 0;
  width: 4px;
  border-radius: 0.25rem 0 0 0.25rem;
  background: var(--template-accent);
}

.gallery-card:hover {
  background: var(--surface-high);
}

.gallery-card--selected {
  background: var(--surface-high);
}

.gallery-card-label {
  font-size: 0.875rem;
  font-weight: var(--weight-title);
  color: var(--color-text-primary);
}

.gallery-card-desc {
  font-size: 0.8125rem;
  color: var(--text-secondary);
  line-height: 1.3;
}

.gallery-card-badge {
  margin-top: auto;
  display: inline-flex;
  padding: 0.125rem 0.5rem;
  border-radius: 4px;
  font-size: 0.75rem;
  font-weight: 500;
  color: var(--color-text-primary);
  background: color-mix(in srgb, var(--template-accent) 18%, transparent);
}
</style>
