<script setup lang="ts">
import { computed, ref } from 'vue';
import { storeToRefs } from 'pinia';
import { usePipelineStore } from '../stores/pipeline';
import { useConfigStore } from '../stores/config';
import { useMcpToolDiscoveryStore } from '../stores/mcp-tool-discovery';
import type { PipelineTemplate } from '../lib/pipeline-templates';
import { getAvailableTemplates, resolveAgents } from '../lib/pipeline-template-runtime';
import TemplateGalleryCard from './TemplateGalleryCard.vue';
import TemplateGalleryIntentForm from './TemplateGalleryIntentForm.vue';
import TemplateGallerySelection from './TemplateGallerySelection.vue';

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
  availableTemplates.value.find((template) => template.id === selectedTemplateId.value) ?? null,
);

const canInstantiate = computed(() => selectedTemplate.value !== null);

function selectTemplate(template: PipelineTemplate): void {
  selectedTemplateId.value = template.id;
}

async function handleInstantiate(): Promise<void> {
  if (selectedTemplate.value && canInstantiate.value && hasIntentAgent.value) {
    await store.applyTemplate(selectedTemplate.value);
  }
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
  } finally {
    intentSubmitting.value = false;
  }
}

function handleIntentInput(value: string): void {
  intentDraft.value = value;
  if (intentError.value) {
    intentError.value = '';
  }
}
</script>

<template>
  <div class="template-gallery flex flex-1 flex-col items-center justify-center px-8">
    <TemplateGalleryIntentForm
      :error="intentError"
      :has-intent-agent="hasIntentAgent"
      :model-value="intentDraft"
      :submitting="intentSubmitting"
      @submit="handleIntentSubmit"
      @update:model-value="handleIntentInput"
    />

    <section class="gallery-card-section">
      <div class="mt-6 mb-3 text-[11px] font-semibold uppercase tracking-wider text-text-tertiary">
        or choose a template
      </div>
      <p v-if="!hasIntentAgent" class="gallery-agent-warning" role="alert">
        Configure an agent before creating a pipeline.
      </p>
      <div class="gallery-cards w-full max-w-[800px]">
        <TemplateGalleryCard
          v-for="template in availableTemplates"
          :key="template.id"
          :selected="selectedTemplateId === template.id"
          :template="template"
          @select="selectTemplate(template)"
        />
      </div>
    </section>

    <TemplateGallerySelection
      v-if="selectedTemplate"
      :can-create="canInstantiate"
      :has-intent-agent="hasIntentAgent"
      :template="selectedTemplate"
      @create="handleInstantiate"
    />
  </div>
</template>

<style scoped>
.template-gallery {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 1.5rem;
  padding-block: 1.5rem;
  height: 100%;
  overflow-y: auto;
}

.gallery-card-section {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.75rem;
  width: 100%;
  text-align: center;
}

.gallery-cards {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 180px));
  gap: 0.75rem;
  justify-content: center;
}

.gallery-agent-warning {
  margin: 0;
  font-size: 0.75rem;
  color: var(--terracotta);
  text-align: center;
}
</style>
