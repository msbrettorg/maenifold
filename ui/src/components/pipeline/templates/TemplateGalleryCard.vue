<script setup lang="ts">
// A single template tile in the gallery. Presentational: it renders the
// template's icon/label/description and a capability badge, and emits `select`
// when actionable. Capability-gated (enterprise) templates that are not yet
// available render disabled with a "requires tools" hint.
import { computed } from 'vue';
import {
  Brain,
  Briefcase,
  FlaskConical,
  Globe,
  PenTool,
  Sparkles,
} from '@lucide/vue';
import type { PipelineTemplate } from '../../../lib/templates/pipelineTemplates';
import type { TemplateAvailability } from '../../../lib/templates/templateRuntime';

const props = defineProps<{
  template: PipelineTemplate;
  availability: TemplateAvailability;
}>();

const emit = defineEmits<{ (event: 'select', template: PipelineTemplate): void }>();

// Kebab icon key → Lucide component. A small explicit map keeps the bundle
// tree-shakeable and avoids a dynamic-import indirection.
const ICONS: Record<string, typeof Sparkles> = {
  'flask-conical': FlaskConical,
  brain: Brain,
  'pen-tool': PenTool,
  globe: Globe,
  briefcase: Briefcase,
};

const iconComponent = computed(() => ICONS[props.template.icon] ?? Sparkles);
const isAvailable = computed(() => props.availability === 'available');
const isPending = computed(() => props.availability === 'discovery-pending');

function onSelect(): void {
  if (isAvailable.value) emit('select', props.template);
}
</script>

<template>
  <button
    type="button"
    class="tpl-card"
    :class="{ unavailable: !isAvailable }"
    :disabled="!isAvailable"
    @click="onSelect"
  >
    <span class="tpl-icon">
      <component :is="iconComponent" :size="18" :stroke-width="2" />
    </span>
    <span class="tpl-body">
      <span class="tpl-head">
        <strong class="tpl-label">{{ template.label }}</strong>
        <span v-if="template.category === 'enterprise'" class="tpl-badge">Enterprise</span>
      </span>
      <span class="tpl-desc">{{ template.description }}</span>
      <span v-if="isPending" class="tpl-hint">Requires tools that aren’t available yet.</span>
      <span v-else-if="!isAvailable" class="tpl-hint">Required tools are not connected.</span>
    </span>
  </button>
</template>

<style scoped>
.tpl-card {
  display: flex;
  gap: 0.65rem;
  width: 100%;
  padding: 0.75rem 0.8rem;
  border: 1px solid var(--border-color);
  border-radius: 10px;
  background: color-mix(in srgb, var(--bg-main) 96%, var(--bg-sidebar));
  color: var(--text-primary);
  cursor: pointer;
  text-align: left;
  transition: background 0.12s ease, border-color 0.12s ease, transform 0.08s ease;
}

.tpl-card:hover:not(:disabled) {
  background: var(--bg-hover);
  border-color: color-mix(in srgb, var(--text-accent) 30%, var(--border-color));
}

.tpl-card.unavailable {
  cursor: default;
  opacity: 0.6;
}

.tpl-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  width: 34px;
  height: 34px;
  border-radius: 8px;
  background: color-mix(in srgb, var(--text-accent) 12%, transparent);
  color: var(--text-accent);
}

.tpl-body {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  min-width: 0;
}

.tpl-head {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
}

.tpl-label {
  font-size: 0.88rem;
  font-weight: 600;
}

.tpl-badge {
  padding: 0.05rem 0.35rem;
  border-radius: 5px;
  font-size: 0.62rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.03em;
  background: color-mix(in srgb, var(--node-accent-thinking) 22%, transparent);
  color: color-mix(in srgb, var(--node-accent-thinking) 85%, var(--text-primary));
}

.tpl-desc {
  font-size: 0.78rem;
  color: var(--text-secondary);
  line-height: 1.35;
}

.tpl-hint {
  font-size: 0.72rem;
  color: var(--text-muted);
}
</style>
