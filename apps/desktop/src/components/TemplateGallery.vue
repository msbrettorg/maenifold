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

const topicInput = ref('');
const accountInput = ref('');
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

const needsAccount = computed(() => selectedTemplate.value?.id === 'deal-preparation');

const canInstantiate = computed(() => {
  if (!selectedTemplate.value) return false;
  if (!topicInput.value.trim() && !needsAccount.value) return false;
  if (needsAccount.value && !accountInput.value.trim()) return false;
  return true;
});

function selectTemplate(template: PipelineTemplate): void {
  selectedTemplateId.value = template.id;
}

function handleInstantiate(): void {
  if (!selectedTemplate.value || !canInstantiate.value) return;

  const agents = resolveAgents(agentNames.value);
  const variables = {
    ...agents,
    topic: topicInput.value.trim(),
    accountName: accountInput.value.trim(),
  };

  store.applyTemplate(selectedTemplate.value, variables);
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
      <template v-if="needsAccount">
        <label class="gallery-field">
          <span class="gallery-field-label">Account name</span>
          <input
            v-model="accountInput"
            type="text"
            class="gallery-field-input"
            placeholder="e.g. Contoso Ltd"
            @keydown.enter="handleInstantiate"
          />
        </label>
      </template>
      <template v-else>
        <label class="gallery-field">
          <span class="gallery-field-label">Topic or question</span>
          <input
            v-model="topicInput"
            type="text"
            class="gallery-field-input"
            :placeholder="`e.g. ${selectedTemplate.description}`"
            @keydown.enter="handleInstantiate"
          />
        </label>
      </template>

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
  border: 1px solid var(--border-color);
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
  border-top: 1px solid var(--border-color);
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

.gallery-field-input {
  padding: 0.5rem 0.75rem;
  border: 1px solid var(--border-color);
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
