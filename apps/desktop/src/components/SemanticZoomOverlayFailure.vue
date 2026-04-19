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
  <div v-if="failureReason" class="zoom-failure-alert" role="alert">
    <strong>Failure reason</strong>
    <pre
      class="zoom-failure-pre"
      :class="{ 'zoom-failure-pre--collapsed': isLong && !isExpanded }"
    >{{ failureReason }}</pre>
    <Button
      v-if="isLong"
      class="zoom-failure-toggle"
      variant="link"
      size="sm"
      type="button"
      @click="isExpanded = !isExpanded"
    >
      {{ isExpanded ? 'Show less' : 'Show more' }}
    </Button>
  </div>
</template>

<style scoped>
.zoom-failure-alert {
  margin: var(--space-3) var(--space-4) 0;
  padding: var(--space-3);
  border-radius: 4px;
  background: color-mix(in srgb, var(--color-status-abandoned) 10%, var(--color-neutral-background-3, var(--card, var(--surface-container))));
  color: var(--color-status-abandoned);
}

.zoom-failure-alert strong {
  display: block;
  font-family: var(--font-display);
  font-size: var(--text-caption);
  font-weight: var(--weight-title);
  margin-bottom: var(--space-1);
}

.zoom-failure-pre {
  margin: 0;
  font-family: var(--font-code);
  font-size: var(--text-tiny);
  line-height: 1.5;
  white-space: pre-wrap;
  word-break: break-word;
  color: inherit;
  max-height: 160px;
  overflow-y: auto;
}

.zoom-failure-pre--collapsed {
  max-height: 3em;
  overflow: hidden;
}

.zoom-failure-toggle {
  margin-top: var(--space-1);
  padding: 0;
  color: var(--color-status-abandoned);
  font-family: var(--font-body);
  font-size: var(--text-tiny);
  opacity: 0.8;
}

.zoom-failure-toggle:hover {
  opacity: 1;
  text-decoration: underline;
}
</style>
