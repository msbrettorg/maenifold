<script setup lang="ts">
import { computed, ref } from 'vue';
import { storeToRefs } from 'pinia';
import { usePipelineStore } from '../stores/pipeline';
import { useConfigStore } from '../stores/config';
import { useMcpToolDiscoveryStore } from '../stores/mcp-tool-discovery';
import type { PipelineTemplate } from '../lib/pipeline-templates';
import { getAvailableTemplates, resolveAgents } from '../lib/pipeline-template-runtime';

const store = usePipelineStore();
const configStore = useConfigStore();
const discoveryStore = useMcpToolDiscoveryStore();
const { agentNames } = storeToRefs(configStore);

const selectedTemplateId = ref<string | null>(null);
const intentDraft = ref('');
const intentError = ref('');
const intentSubmitting = ref(false);

const discoveredToolNames = computed<ReadonlySet<string>>(
  () => new Set(discoveryStore.allTools.map((tool) => tool.name)),
);

const availableTemplates = computed(() =>
  getAvailableTemplates(discoveredToolNames.value),
);
const hasIntentAgent = computed(() => resolveAgents(agentNames.value).primaryAgent.trim().length > 0);

const selectedTemplate = computed<PipelineTemplate | null>(() =>
  availableTemplates.value.find((t) => t.id === selectedTemplateId.value) ?? null,
);

const canInstantiate = computed(() => selectedTemplate.value !== null);

function selectTemplate(template: PipelineTemplate): void {
  selectedTemplateId.value = template.id;
}

async function handleInstantiate(): Promise<void> {
  if (!selectedTemplate.value || !canInstantiate.value || !hasIntentAgent.value) return;

  await store.applyTemplate(selectedTemplate.value);
}

async function handleIntentSubmit(): Promise<void> {
  if (intentSubmitting.value) {
    return;
  }

  intentError.value = '';
  intentSubmitting.value = true;
  try {
    const created = await store.createIntentPipeline(intentDraft.value);
    if (!created) {
      if (intentDraft.value.trim().length === 0) {
        intentError.value = 'Enter an intent to create a pipeline.';
      }
      return;
    }

    intentDraft.value = '';
    intentError.value = '';
  } finally {
    intentSubmitting.value = false;
  }
}

function handleIntentInput(): void {
  if (intentError.value) {
    intentError.value = '';
  }
}
</script>

<template>
  <div class="template-gallery">
    <header class="gallery-header">
      <h3 class="gallery-title">What would you like to do?</h3>
    </header>

    <section class="gallery-hero">
      <form class="gallery-intent-form" @submit.prevent="handleIntentSubmit">
        <label class="gallery-intent-label" for="pipeline-intent-input">
          Start with an intent
        </label>
        <input
          id="pipeline-intent-input"
          v-model="intentDraft"
          type="text"
          class="gallery-intent-input"
          placeholder="What do you want to do?"
          autocomplete="off"
          :disabled="intentSubmitting"
          :aria-invalid="intentError.length > 0"
          @input="handleIntentInput"
        />
        <p v-if="intentError" class="gallery-intent-error" role="alert">
          {{ intentError }}
        </p>
        <p class="gallery-intent-hint">
          Press Enter to start designing your pipeline.
        </p>
        <p v-if="!hasIntentAgent" class="gallery-agent-warning" role="alert">
          Configure an agent before creating a pipeline.
        </p>
      </form>
    </section>

    <section class="gallery-card-section">
      <div class="gallery-card-section-header">
        <span class="gallery-card-section-label">Or start from an intent card</span>
      </div>
      <p v-if="!hasIntentAgent" class="gallery-agent-warning" role="alert">
        Configure an agent before creating a pipeline.
      </p>
      <div class="gallery-cards">
        <button
          v-for="template in availableTemplates"
          :key="template.id"
          type="button"
          :class="['gallery-card', { 'gallery-card--selected': selectedTemplateId === template.id }]"
          @click="selectTemplate(template)"
        >
          <span class="gallery-card-icon">{{ template.icon }}</span>
          <span class="gallery-card-label">{{ template.label }}</span>
          <span class="gallery-card-desc">{{ template.description }}</span>
          <span v-if="template.category === 'enterprise'" class="gallery-card-badge">Enterprise</span>
        </button>
      </div>
    </section>

    <div v-if="selectedTemplate" class="gallery-input-section">
      <div
        v-if="selectedTemplate.runInputs.length > 0"
        class="gallery-runtime-inputs"
      >
        <span class="gallery-field-label">Run-time inputs</span>
        <ul class="gallery-runtime-input-list">
          <li
            v-for="input in selectedTemplate.runInputs"
            :key="input.key"
            class="gallery-runtime-input-item"
          >
            <span class="gallery-runtime-input-name">
              {{ input.label }}
              <span v-if="input.required" class="gallery-runtime-input-required">*</span>
            </span>
            <span v-if="input.description" class="gallery-runtime-input-desc">
              {{ input.description }}
            </span>
          </li>
        </ul>
        <p class="gallery-runtime-note">
          These values are requested when you run the pipeline, not when you create it.
        </p>
      </div>

      <div class="gallery-actions">
        <button
          type="button"
          class="btn btn-primary"
          :disabled="!canInstantiate || !hasIntentAgent"
          @click="handleInstantiate"
        >
          Create Pipeline
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.template-gallery {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 1.5rem;
  padding: 1.5rem;
  height: 100%;
  overflow-y: auto;
}

.gallery-header {
  display: flex;
  align-items: center;
  justify-content: center;
  width: min(100%, 960px);
  text-align: center;
}

.gallery-title {
  margin: 0;
  font-size: var(--text-lg);
  font-weight: 600;
  color: var(--text-primary);
}

.gallery-hero {
  display: flex;
  justify-content: center;
  padding: 1rem 0 0.25rem;
}

.gallery-intent-form {
  width: min(100%, 720px);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.75rem;
}

.gallery-intent-label {
  font-size: var(--text-sm);
  font-weight: 600;
  color: var(--text-secondary);
}

.gallery-intent-input {
  width: 100%;
  min-height: 4rem;
  padding: 1rem 1.25rem;
  border: 1px solid color-mix(in srgb, var(--color-accent, #4d8eff) 35%, var(--border-default));
  border-radius: 18px;
  background: color-mix(in srgb, var(--surface-container-highest) 84%, var(--bg-main));
  color: var(--text-primary);
  font-size: clamp(1.05rem, 1rem + 0.35vw, 1.35rem);
  line-height: 1.4;
  box-shadow: 0 18px 40px color-mix(in srgb, var(--color-accent, #4d8eff) 10%, transparent);
}

.gallery-intent-input::placeholder {
  color: var(--text-muted);
}

.gallery-intent-input:focus {
  outline: none;
  border-color: var(--color-accent, #4d8eff);
  box-shadow:
    0 0 0 3px color-mix(in srgb, var(--color-accent, #4d8eff) 18%, transparent),
    0 18px 40px color-mix(in srgb, var(--color-accent, #4d8eff) 12%, transparent);
}

.gallery-intent-hint {
  margin: 0;
  font-size: var(--text-sm);
  color: var(--text-muted);
  text-align: center;
}

.gallery-intent-error {
  margin: 0;
  font-size: var(--text-sm);
  color: var(--color-status-stuck, #e57373);
  text-align: center;
}

.gallery-card-section {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.75rem;
  width: min(100%, 960px);
  text-align: center;
}

.gallery-card-section-header {
  display: flex;
  justify-content: center;
}

.gallery-card-section-label {
  font-size: var(--text-xs);
  font-weight: 600;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--text-muted);
}

.gallery-cards {
  display: grid;
  width: min(100%, 960px);
  grid-template-columns: repeat(auto-fit, minmax(180px, 180px));
  gap: 0.75rem;
  justify-content: center;
}

.gallery-card {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.25rem;
  padding: 0.75rem;
  border: 1px solid var(--border-default);
  border-radius: 8px;
  background: var(--surface-container-highest);
  cursor: pointer;
  text-align: left;
  transition: border-color 0.15s, background 0.15s;
}

.gallery-card:hover {
  background: var(--bg-surface);
  border-color: var(--text-muted);
}

.gallery-card--selected {
  border-color: var(--color-accent, #4d8eff);
  background: color-mix(in srgb, var(--color-accent, #4d8eff) 6%, var(--bg-surface));
}

.gallery-card-icon {
  font-size: 1.5rem;
  line-height: 1;
}

.gallery-card-label {
  font-size: var(--text-sm);
  font-weight: 600;
  color: var(--text-primary);
}

.gallery-card-desc {
  font-size: var(--text-xs);
  color: var(--text-muted);
  line-height: 1.3;
}

.gallery-card-badge {
  margin-top: 0.25rem;
  display: inline-flex;
  padding: 1px 6px;
  border-radius: 999px;
  font-size: var(--text-xs);
  font-weight: 500;
  color: var(--color-accent, #4d8eff);
  background: color-mix(in srgb, var(--color-accent, #4d8eff) 10%, transparent);
}

.gallery-input-section {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  padding-top: 0.5rem;
  width: min(100%, 720px);
  border-top: 1px solid var(--border-default);
}

.gallery-field {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.gallery-field-label {
  font-size: var(--text-xs);
  font-weight: 500;
  color: var(--text-secondary);
}

.gallery-runtime-inputs {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.gallery-runtime-input-list {
  margin: 0;
  padding-left: 1rem;
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.gallery-runtime-input-item {
  color: var(--text-primary);
  font-size: var(--text-sm);
}

.gallery-runtime-input-name {
  font-weight: 500;
}

.gallery-runtime-input-required {
  color: var(--color-status-stuck, #e57373);
}

.gallery-runtime-input-desc,
.gallery-runtime-note {
  color: var(--text-muted);
  font-size: var(--text-xs);
}

.gallery-field-input {
  padding: 0.5rem 0.75rem;
  border: 1px solid var(--border-default);
  border-radius: 6px;
  background: var(--bg-main);
  color: var(--text-primary);
  font-size: var(--text-sm);
}

.gallery-field-input:focus {
  outline: none;
  border-color: var(--color-accent, #4d8eff);
}

.gallery-actions {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.75rem;
}

.gallery-agent-warning {
  font-size: var(--text-xs);
  color: var(--color-status-stuck, #e57373);
  text-align: center;
}
</style>
