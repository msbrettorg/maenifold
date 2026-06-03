<script setup lang="ts">
// Template gallery — the empty-stage entry point for the Pipelines tab. Shows
// the intent track (free-form) plus the capability-gated template track, and
// instantiates REAL server-side pipelines through the gallery controller.
// Rendered by PipelinesView only when no pipeline is selected.
import { computed, onMounted, ref } from 'vue';
import { LayoutTemplate } from '@lucide/vue';
import { useTemplateGallery } from './useTemplateGallery';
import TemplateGalleryCard from './TemplateGalleryCard.vue';
import TemplateIntentForm from './TemplateIntentForm.vue';
import TemplateInstantiatePanel from './TemplateInstantiatePanel.vue';
import type { PipelineTemplate, TemplateInputValues } from '../../../lib/templates/pipelineTemplates';
import { PIPELINE_TEMPLATES } from '../../../lib/templates/pipelineTemplates';

const gallery = useTemplateGallery();
const {
  creating,
  createError,
  hasAgents,
  discoveryStatus,
  availableTemplates,
  availabilityOf,
  ensureDiscovery,
  createFromTemplate,
  createFromIntent,
} = gallery;

// The template the user picked to instantiate (null = gallery view).
const selected = ref<PipelineTemplate | null>(null);

const agentDisabledReason = computed(() =>
  hasAgents.value ? null : 'Configure an agent in settings to create a pipeline.'
);

// Capability-gated templates fail CLOSED: only render templates that are
// actually available. Hidden (gated) templates must NOT appear — not even
// disabled — so we never advertise a pipeline the server can't run. We do
// surface a subtle count once discovery is ready so the gating is explained.
const hiddenCount = computed(() => PIPELINE_TEMPLATES.length - availableTemplates.value.length);
const showHiddenHint = computed(
  () => discoveryStatus.value === 'ready' && hiddenCount.value > 0
);

onMounted(() => {
  void ensureDiscovery();
});

function onSelect(template: PipelineTemplate): void {
  createError.value = null;
  selected.value = template;
}

function onBack(): void {
  selected.value = null;
  createError.value = null;
}

async function onTemplateSubmit(values: TemplateInputValues): Promise<void> {
  if (!selected.value) return;
  const ok = await createFromTemplate(selected.value, values);
  if (ok) selected.value = null;
}

async function onIntentSubmit(intent: string): Promise<void> {
  await createFromIntent(intent);
}
</script>

<template>
  <div class="gallery">
    <header class="gallery-header">
      <LayoutTemplate :size="18" :stroke-width="2" />
      <div class="gallery-heading">
        <strong>Start a pipeline</strong>
        <span>Describe an intent, or instantiate a template.</span>
      </div>
    </header>

    <div class="gallery-scroll">
      <!-- Selected template → input panel; otherwise the gallery. -->
      <TemplateInstantiatePanel
        v-if="selected"
        :template="selected"
        :creating="creating"
        :error="createError"
        @submit="onTemplateSubmit"
        @back="onBack"
      />

      <template v-else>
        <TemplateIntentForm
          :creating="creating"
          :disabled="!hasAgents"
          :disabled-reason="agentDisabledReason"
          :error="createError"
          @submit="onIntentSubmit"
        />

        <section class="gallery-templates" aria-label="Pipeline templates">
          <h3 class="gallery-subtitle">Templates</h3>
          <p v-if="availableTemplates.length === 0" class="gallery-empty">
            No templates available yet.
          </p>
          <div class="gallery-grid">
            <TemplateGalleryCard
              v-for="template in availableTemplates"
              :key="template.id"
              :template="template"
              :availability="availabilityOf(template)"
              @select="onSelect"
            />
          </div>
          <p v-if="showHiddenHint" class="gallery-empty">
            {{ hiddenCount }} template{{ hiddenCount === 1 ? '' : 's' }} hidden until the
            required tools are connected.
          </p>
        </section>
      </template>
    </div>
  </div>
</template>

<style scoped>
.gallery {
  display: flex;
  flex-direction: column;
  width: 100%;
  min-width: 0;
  min-height: 0;
}

.gallery-header {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  padding: 0.85rem 1rem;
  border-bottom: 1px solid var(--border-color);
  color: var(--text-accent);
}

.gallery-heading {
  display: flex;
  flex-direction: column;
  gap: 0.1rem;
}

.gallery-heading strong {
  font-size: 0.92rem;
  color: var(--text-primary);
}

.gallery-heading span {
  font-size: 0.78rem;
  color: var(--text-secondary);
}

.gallery-scroll {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 1rem;
  padding: 1rem;
  overflow-y: auto;
}

.gallery-templates {
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
}

.gallery-subtitle {
  margin: 0;
  font-size: 0.74rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.03em;
  color: var(--text-muted);
}

.gallery-empty {
  margin: 0;
  font-size: 0.8rem;
  color: var(--text-muted);
}

.gallery-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
  gap: 0.65rem;
}
</style>
