<script setup lang="ts">
import { computed } from 'vue';
import VueJsonPretty from 'vue-json-pretty';
// audit-ok: library-css — vue-json-pretty ships its colorization in a single CSS file.
import 'vue-json-pretty/lib/styles.css';

type JsonValue = string | number | boolean | unknown[] | Record<string, unknown> | null;

const props = defineProps<{ payload: unknown }>();

// Coerce the unknown payload into a vue-json-pretty-compatible value.
// Primitives are passed through; undefined collapses to null.
const safeData = computed<JsonValue>(() => {
  const value = props.payload;
  if (value === undefined) return null;
  if (
    value === null ||
    typeof value === 'string' ||
    typeof value === 'number' ||
    typeof value === 'boolean' ||
    Array.isArray(value) ||
    typeof value === 'object'
  ) {
    return value as JsonValue;
  }
  return String(value);
});
</script>

<template>
  <div class="px-3 pb-3 pt-2 pl-8 overflow-x-auto text-xs">
    <VueJsonPretty
      :data="safeData"
      :deep="3"
      :show-line="false"
      :show-double-quotes="true"
      :show-length="true"
    />
  </div>
</template>
