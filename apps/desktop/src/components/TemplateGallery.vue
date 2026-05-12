<script setup lang="ts">
import { computed, ref } from 'vue';
import { storeToRefs } from 'pinia';
import { usePipelineStore } from '../stores/pipeline';
import { useConfigStore } from '../stores/config';
import { useMcpToolDiscoveryStore } from '../stores/mcp-tool-discovery';
import type { PipelineTemplate, TemplateInputValues } from '../lib/pipeline-templates';
import { getAvailableTemplates, resolveAgents } from '../lib/pipeline-template-runtime';
import TemplateGalleryCard from './TemplateGalleryCard.vue';
import TemplateGalleryIntentForm from './TemplateGalleryIntentForm.vue';
import TemplateGallerySelection from './TemplateGallerySelection.vue';
import { Card, CardContent } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';

const store = usePipelineStore();
const configStore = useConfigStore();
const discoveryStore = useMcpToolDiscoveryStore();
const { agentNames } = storeToRefs(configStore);

type TemplateGalleryIntentFormExpose = { focusIntentInput: () => void };

const intentFormRef = ref<TemplateGalleryIntentFormExpose | null>(null);
const selectedTemplateId = ref<string | null>(null);
const intentDraft = ref('');
const intentError = ref('');
const intentSubmitting = ref(false);
const creationNoticeVisible = ref(false);

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
  creationNoticeVisible.value = false;
}

async function handleInstantiate(inputValues: TemplateInputValues): Promise<void> {
  if (selectedTemplate.value && canInstantiate.value && hasIntentAgent.value) {
    await store.applyTemplate(selectedTemplate.value, inputValues);
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
    creationNoticeVisible.value = false;
  } finally {
    intentSubmitting.value = false;
  }
}

function handleIntentInput(value: string): void {
  intentDraft.value = value;
  if (intentError.value) {
    intentError.value = '';
  }
  creationNoticeVisible.value = false;
}

function resetForNewPipeline(): void {
  selectedTemplateId.value = null;
  intentError.value = '';
  creationNoticeVisible.value = true;
  intentFormRef.value?.focusIntentInput();
}

defineExpose({ resetForNewPipeline });
</script>

<template>
  <ScrollArea class="flex-1 h-full">
  <div class="flex flex-col items-center gap-6 px-8 py-6">
    <TemplateGalleryIntentForm
      ref="intentFormRef"
      :error="intentError"
      :has-intent-agent="hasIntentAgent"
      :model-value="intentDraft"
      :submitting="intentSubmitting"
      @submit="handleIntentSubmit"
      @update:model-value="handleIntentInput"
    />
    <p
      v-if="creationNoticeVisible"
      class="m-0 -mt-4 text-[0.75rem] font-medium text-brand"
      role="status"
    >
      Ready to create a new pipeline.
    </p>

    <section class="flex flex-col items-center gap-3 w-full text-center">
      <div class="mt-6 mb-3 text-[11px] font-semibold tracking-wider text-text-tertiary">
        Or choose a template
      </div>
      <p v-if="!hasIntentAgent" class="m-0 text-[0.75rem] text-danger text-center" role="alert">
        Configure an agent before creating a pipeline.
      </p>
      <Card class="w-full max-w-[800px] bg-transparent border-border-subtle/50">
        <CardContent class="grid grid-cols-2 gap-3 lg:grid-cols-4">
          <TemplateGalleryCard
            v-for="template in availableTemplates"
            :key="template.id"
            :selected="selectedTemplateId === template.id"
            :template="template"
            @select="selectTemplate(template)"
          />
        </CardContent>
      </Card>
    </section>

    <TemplateGallerySelection
      v-if="selectedTemplate"
      :can-create="canInstantiate"
      :has-intent-agent="hasIntentAgent"
      :template="selectedTemplate"
      @create="handleInstantiate"
    />
  </div>
  </ScrollArea>
</template>
