<script setup lang="ts">
import { computed, ref } from 'vue';
import { Play } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { Badge } from '@/components/ui/badge';

interface PipelineCreateFormSubmit {
  name?: string;
  description?: string;
  inputSchema?: Record<string, unknown>;
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
const inputSchemaText = ref('');
const showFallbackWarning = ref(false);
const sanitizedName = computed(() => sanitizePipelineName(pipelineName.value));
const parsedInputSchema = computed(() => parseInputSchema(inputSchemaText.value));

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

function parseInputSchema(value: string): { ok: true; value?: Record<string, unknown> } | { ok: false; message: string } {
  const trimmed = value.trim();
  if (!trimmed) return { ok: true };
  try {
    const parsed = JSON.parse(trimmed) as unknown;
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
      return { ok: false, message: 'Schema must be a JSON object.' };
    }
    return { ok: true, value: parsed as Record<string, unknown> };
  } catch (error) {
    return { ok: false, message: error instanceof Error ? error.message : 'Invalid JSON.' };
  }
}

function submitCreateForm(): void {
  if (!parsedInputSchema.value.ok) return;
  const name = sanitizedName.value;
  const description = pipelineDescription.value.trim();
  showFallbackWarning.value = name.length === 0;
  emit('submit', {
    name: name || undefined,
    description: description || undefined,
    inputSchema: parsedInputSchema.value.value,
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

    <div class="flex min-w-[18rem] flex-[1.4] flex-col gap-1.5">
      <div class="flex items-center justify-between gap-2">
        <Label for="pipeline-create-input-schema" class="text-xs font-medium text-text-secondary">
          Input schema
        </Label>
        <Badge :variant="parsedInputSchema.ok ? 'secondary' : 'destructive'">
          {{ parsedInputSchema.ok ? 'Valid JSON' : 'Invalid JSON' }}
        </Badge>
      </div>
      <Textarea
        id="pipeline-create-input-schema"
        v-model="inputSchemaText"
        class="min-h-20 py-2 font-mono text-xs"
        placeholder='{"type":"object","required":["project_name"],"properties":{"project_name":{"type":"string"}}}'
        :disabled="disabled"
        :aria-invalid="parsedInputSchema.ok ? undefined : 'true'"
      />
      <p class="m-0 text-xs" :class="parsedInputSchema.ok ? 'text-text-tertiary' : 'text-danger'">
        {{ parsedInputSchema.ok ? 'Optional JSON Schema object.' : parsedInputSchema.message }}
      </p>
    </div>

    <Button type="submit" class="mt-[1.55rem] inline-flex items-center gap-[0.4rem]" :disabled="disabled">
      <Play :size="16" aria-hidden="true" />
      <span>Create</span>
    </Button>
  </form>
</template>
