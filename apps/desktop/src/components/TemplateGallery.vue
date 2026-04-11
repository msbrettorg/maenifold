<script setup lang="ts">
import { computed, ref } from 'vue';
import { storeToRefs } from 'pinia';
import { usePipelineStore } from '../stores/pipeline';
import { useConfigStore } from '../stores/config';
import { useMcpToolDiscoveryStore } from '../stores/mcp-tool-discovery';
import type { PipelineTemplate } from '../lib/pipeline-templates';
import {
  getAvailableTemplates,
  resolveAgents,
} from '../lib/pipeline-template-runtime';

const emit = defineEmits<{
  (e: 'switchToAdvanced'): void;
}>();

const store = usePipelineStore();
const configStore = useConfigStore();
const discoveryStore = useMcpToolDiscoveryStore();
const { agentNames } = storeToRefs(configStore);

const selectedTemplateId = ref<string | null>(null);

const discoveredToolNames = computed<ReadonlySet<string>>(
  () => new Set(discoveryStore.allTools.map((tool) => tool.name)),
);

const availableTemplates = computed(() =>
  getAvailableTemplates(discoveredToolNames.value),
);

const selectedTemplate = computed<PipelineTemplate | null>(() =>
  availableTemplates.value.find((t) => t.id === selectedTemplateId.value) ?? null,
);

const canInstantiate = computed(() => selectedTemplate.value !== null);

function selectTemplate(template: PipelineTemplate): void {
  selectedTemplateId.value = template.id;
}

function handleInstantiate(): void {
  if (!selectedTemplate.value || !canInstantiate.value) return;

  const agents = resolveAgents(agentNames.value);
  store.applyTemplate(selectedTemplate.value, agents);
}
</script>

<template>
  <div class="template-gallery">
    <header class="gallery-header">
      <h3 class="gallery-title">What would you like to do?</h3>
      <button
        type="button"
        class="btn btn-ghost gallery-advanced-btn"
        @click="emit('switchToAdvanced')"
      >
        Advanced →
      </button>
    </header>

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
        <span v-if="agentNames.length === 0" class="gallery-agent-warning">
          No agents configured — nodes will need manual agent assignment.
        </span>
        <button
          type="button"
          class="btn btn-primary"
          :disabled="!canInstantiate"
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
  gap: 1rem;
  padding: 1rem;
  height: 100%;
  overflow-y: auto;
}

.gallery-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.gallery-title {
  margin: 0;
  font-size: var(--text-lg);
  font-weight: 600;
  color: var(--text-primary);
}

.gallery-advanced-btn {
  font-size: var(--text-sm);
  color: var(--text-muted);
}

.gallery-cards {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 0.75rem;
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
  justify-content: flex-end;
  gap: 0.75rem;
}

.gallery-agent-warning {
  font-size: var(--text-xs);
  color: var(--color-status-stuck, #e57373);
}
</style>
