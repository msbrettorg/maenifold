<script setup lang="ts">
import type { PipelineTemplate } from '../lib/pipeline-templates';

const ACCENT_BY_TEMPLATE_ID: Record<string, string> = {
  'research-and-verify': '#4d8eff',
  'think-and-critique': '#d0bcff',
  'draft-and-refine': '#F59E0B',
  'explore-and-synthesize': '#4edea3',
  'deal-preparation': '#F59E0B',
};

const props = defineProps<{
  selected: boolean;
  template: PipelineTemplate;
}>();

defineEmits<{
  select: [];
}>();
</script>

<template>
  <button
    type="button"
    :class="['gallery-card', { 'gallery-card--selected': props.selected }]"
    :style="{ '--template-accent': ACCENT_BY_TEMPLATE_ID[props.template.id] ?? '#4d8eff' }"
    @click="$emit('select')"
  >
    <span class="gallery-card-icon">{{ props.template.icon }}</span>
    <span class="gallery-card-label">{{ props.template.label }}</span>
    <span class="gallery-card-desc">{{ props.template.description }}</span>
    <span v-if="props.template.category === 'enterprise'" class="gallery-card-badge">Enterprise</span>
  </button>
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
  border: 0;
  border-radius: 0.25rem;
  background: #201f1f;
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
  background: #2a2a2a;
}

.gallery-card--selected {
  background: #2a2a2a;
}

.gallery-card-icon {
  font-size: var(--text-headline);
  line-height: 1;
}

.gallery-card-label {
  font-size: 0.875rem;
  font-weight: var(--weight-title);
  color: #e5e2e1;
}

.gallery-card-desc {
  font-size: 0.8125rem;
  color: #c2c6d6;
  line-height: 1.3;
}

.gallery-card-badge {
  margin-top: auto;
  display: inline-flex;
  padding: 0.125rem 0.5rem;
  border-radius: 4px;
  font-size: 0.75rem;
  font-weight: 500;
  color: #e5e2e1;
  background: color-mix(in srgb, var(--template-accent) 18%, transparent);
}
</style>
