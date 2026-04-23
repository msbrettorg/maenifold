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
  <div class="flex flex-1 flex-col items-center gap-6 px-8 py-6 h-full overflow-y-auto">
    <TemplateGalleryIntentForm
      :error="intentError"
      :has-intent-agent="hasIntentAgent"
      :model-value="intentDraft"
      :submitting="intentSubmitting"
      @submit="handleIntentSubmit"
      @update:model-value="handleIntentInput"
    />

    <section class="flex flex-col items-center gap-3 w-full text-center">
      <div class="mt-6 mb-3 text-[11px] font-semibold tracking-wider text-text-tertiary">
        Or choose a template
      </div>
      <p v-if="!hasIntentAgent" class="m-0 text-[0.75rem] text-[var(--terracotta)] text-center" role="alert">
        Configure an agent before creating a pipeline.
      </p>
      <div class="grid grid-cols-[repeat(auto-fit,minmax(180px,180px))] gap-3 justify-center w-full max-w-[800px]">
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
