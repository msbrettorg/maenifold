<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { Button } from '@/components/ui/button';

const props = defineProps<{
  failureReason: string | null;
}>();

const isExpanded = ref(false);
const isLong = computed(() => (props.failureReason?.length ?? 0) > 200);

watch(() => props.failureReason, () => {
  isExpanded.value = false;
});
</script>

<template>
  <!-- audit-ok: dynamic-binding (color-mix blend of status-abandoned with surface fallback chain not expressible as a single token) -->
  <div
    v-if="failureReason"
    class="mt-[var(--space-3)] mx-[var(--space-4)] p-[var(--space-3)] rounded-[4px] text-[var(--color-status-abandoned)]"
    style="background: color-mix(in srgb, var(--color-status-abandoned) 10%, var(--color-neutral-background-3, var(--card, var(--surface-container))));"
    role="alert"
  >
    <strong class="block font-[family-name:var(--font-display)] text-[length:var(--text-caption)] font-[number:var(--weight-title)] mb-[var(--space-1)]">Failure reason</strong>
    <pre
      class="m-0 font-mono text-[length:var(--text-tiny)] leading-[1.5] whitespace-pre-wrap break-words text-inherit max-h-[160px] overflow-y-auto"
      :class="{ 'max-h-[3em] overflow-hidden': isLong && !isExpanded }"
    >{{ failureReason }}</pre>
    <Button
      v-if="isLong"
      class="mt-[var(--space-1)] p-0 text-[var(--color-status-abandoned)] font-[family-name:var(--font-body)] text-[length:var(--text-tiny)] opacity-80 hover:opacity-100 hover:underline"
      variant="link"
      size="sm"
      type="button"
      @click="isExpanded = !isExpanded"
    >
      {{ isExpanded ? 'Show less' : 'Show more' }}
    </Button>
  </div>
</template>
