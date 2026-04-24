<script setup lang="ts">
import { computed } from 'vue';
import VueJsonPretty from 'vue-json-pretty';

type JsonValue = string | number | boolean | unknown[] | Record<string, unknown> | null;

const props = defineProps<{ payload: unknown }>();

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

const extraProps = { deep: 3 };
</script>

<template>
  <div class="px-3 pb-3 pt-2 pl-8 overflow-x-auto text-xs font-mono">
    <VueJsonPretty
      :data="safeData"
      v-bind="extraProps"
      :show-line="false"
      :show-double-quotes="true"
      :show-length="true"
    />
  </div>
</template>
