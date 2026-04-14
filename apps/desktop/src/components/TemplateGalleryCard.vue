<script setup lang="ts">
import type { PipelineTemplate } from '../lib/pipeline-templates';

defineProps<{
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
    :class="['gallery-card', { 'gallery-card--selected': selected }]"
    @click="$emit('select')"
  >
    <span class="gallery-card-icon">{{ template.icon }}</span>
    <span class="gallery-card-label">{{ template.label }}</span>
    <span class="gallery-card-desc">{{ template.description }}</span>
    <span v-if="template.category === 'enterprise'" class="gallery-card-badge">Enterprise</span>
  </button>
</template>

<style scoped>
.gallery-card {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.25rem;
  padding: 0.75rem;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--accent);
  cursor: pointer;
  text-align: left;
  transition: border-color 0.15s, background 0.15s;
}

.gallery-card:hover {
  background: var(--muted);
  border-color: var(--muted-foreground);
}

.gallery-card--selected {
  border-color: var(--color-accent, #4d8eff);
  background: color-mix(in srgb, var(--color-accent, #4d8eff) 6%, var(--muted));
}

.gallery-card-icon {
  font-size: var(--text-headline);
  line-height: 1;
}

.gallery-card-label {
  font-size: var(--text-caption);
  font-weight: var(--weight-title);
  color: var(--foreground);
}

.gallery-card-desc {
  font-size: var(--text-tiny);
  color: var(--muted-foreground);
  line-height: 1.3;
}

.gallery-card-badge {
  margin-top: 0.25rem;
  display: inline-flex;
  padding: 1px 6px;
  border-radius: 999px;
  font-size: var(--text-tiny);
  font-weight: var(--weight-display);
  color: var(--color-accent, #4d8eff);
  background: color-mix(in srgb, var(--color-accent, #4d8eff) 10%, transparent);
}
</style>
