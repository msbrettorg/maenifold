<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { AlertCircle } from 'lucide-vue-next';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
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
  <Alert
    v-if="failureReason"
    variant="destructive"
    class="mt-3 mx-4"
    role="alert"
  >
    <AlertCircle />
    <AlertTitle>Failure reason</AlertTitle>
    <AlertDescription>
      <pre
        class="m-0 font-mono text-xs leading-relaxed whitespace-pre-wrap break-words text-inherit overflow-y-auto"
        :class="isLong && !isExpanded ? 'max-h-12 overflow-hidden' : 'max-h-40'"
      >{{ failureReason }}</pre>
      <Button
        v-if="isLong"
        variant="link"
        size="sm"
        class="mt-1 h-auto p-0 text-destructive opacity-80 hover:opacity-100"
        @click="isExpanded = !isExpanded"
      >
        {{ isExpanded ? 'Show less' : 'Show more' }}
      </Button>
    </AlertDescription>
  </Alert>
</template>
