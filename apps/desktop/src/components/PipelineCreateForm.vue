<script setup lang="ts">
import { computed, ref } from 'vue';
import { Play } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';

interface PipelineCreateFormSubmit {
  name?: string;
  description?: string;
}

const props = withDefaults(defineProps<{
  disabled?: boolean;
}>(), {
  disabled: false,
});

const emit = defineEmits<{
  (e: 'submit', value: PipelineCreateFormSubmit): void;
}>();

const pipelineName = ref('');
const pipelineDescription = ref('');
const showFallbackWarning = ref(false);
const sanitizedName = computed(() => sanitizePipelineName(pipelineName.value));

function sanitizePipelineName(value: string): string {
  const slug = value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9-]+/g, '-')
    .replace(/-+/g, '-')
    .replace(/^-|-$/g, '');
  if (slug.length === 0) return '';
  return /^[a-z]/.test(slug) ? slug : `pipeline-${slug}`;
}

function submitCreateForm(): void {
  const name = sanitizedName.value;
  const description = pipelineDescription.value.trim();
  showFallbackWarning.value = name.length === 0;
  emit('submit', {
    name: name || undefined,
    description: description || undefined,
  });
}
</script>

<template>
  <form
    class="flex flex-wrap items-start gap-3"
    aria-label="Create pipeline details"
    @submit.prevent="submitCreateForm"
  >
    <div class="flex min-w-[12rem] flex-1 flex-col gap-1.5">
      <Label for="pipeline-create-name" class="text-xs font-medium text-text-secondary">
        Name
      </Label>
      <Input
        id="pipeline-create-name"
        v-model="pipelineName"
        type="text"
        placeholder="research-run"
        :disabled="disabled"
        :aria-invalid="showFallbackWarning ? 'true' : undefined"
        aria-describedby="pipeline-create-name-help"
      />
      <p
        id="pipeline-create-name-help"
        class="m-0 text-xs"
        :class="showFallbackWarning ? 'text-danger' : 'text-text-tertiary'"
      >
        {{ showFallbackWarning ? 'No name entered. A pipeline-* ID will be generated.' : 'Used as the saved pipeline ID prefix.' }}
      </p>
    </div>

    <div class="flex min-w-[16rem] flex-[1.4] flex-col gap-1.5">
      <Label for="pipeline-create-description" class="text-xs font-medium text-text-secondary">
        Description
      </Label>
      <Textarea
        id="pipeline-create-description"
        v-model="pipelineDescription"
        class="min-h-9 py-2"
        placeholder="Optional context for this pipeline"
        :disabled="disabled"
      />
      <p class="m-0 text-xs text-text-tertiary">Optional.</p>
    </div>

    <Button type="submit" class="mt-[1.55rem] inline-flex items-center gap-[0.4rem]" :disabled="disabled">
      <Play :size="16" aria-hidden="true" />
      <span>Create</span>
    </Button>
  </form>
</template>
