<script setup lang="ts">
import type { JsonSchema } from '@jsonforms/core';
import { JsonForms } from '@jsonforms/vue';
import { frozenRenderers } from '../lib/jsonforms-theme';
import { schemaFormAjv } from '../lib/schema-form';
import { ScrollArea } from '@/components/ui/scroll-area';

const props = defineProps<{
  ariaLabel?: string;
  'aria-label'?: string;
  data: Record<string, unknown>;
  schema?: JsonSchema;
}>();

defineEmits<{
  change: [event: { data: unknown; errors: unknown[] }];
}>();
</script>

<template>
  <ScrollArea
    class="pr-1 max-h-[min(32rem,60vh)]"
    role="group"
    :aria-label="props.ariaLabel ?? props['aria-label']"
  >
    <JsonForms
      :ajv="schemaFormAjv"
      :data="data"
      :renderers="frozenRenderers"
      :schema="schema"
      validation-mode="ValidateAndShow"
      @change="$emit('change', $event)"
    />
  </ScrollArea>
</template>
