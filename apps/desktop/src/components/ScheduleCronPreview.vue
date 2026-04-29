<script setup lang="ts">
import { computed, toRef, watch } from 'vue';
import { Loader2 } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import { useSchedulePreview } from '@/composables/useSchedulePreview';

const props = defineProps<{
  expression: string;
}>();

const emit = defineEmits<{
  'preview-valid': [valid: boolean];
}>();

const showUtc = defineModel<boolean>('showUtc', { default: false });
const localTimeZone = Intl.DateTimeFormat().resolvedOptions().timeZone || 'Local time';
const { loading, error, cron, description, nextFires, previewIsValid } = useSchedulePreview(
  toRef(props, 'expression'),
);

const heading = computed(() => showUtc.value ? 'Next 5 fires (UTC)' : 'Next 5 fires (Local TZ)');

function formatFire(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: 'medium',
    timeStyle: 'short',
    timeZone: showUtc.value ? 'UTC' : undefined,
  }).format(date);
}

watch(previewIsValid, (valid) => emit('preview-valid', valid), { immediate: true });
</script>

<template>
  <div v-if="expression.trim()" class="rounded-md border bg-muted/30 px-4 py-3 text-sm">
    <div v-if="loading" class="flex items-center gap-2 text-muted-foreground" role="status">
      <Loader2 class="h-4 w-4 animate-spin" />
      Validating...
    </div>

    <p v-else-if="error" class="text-destructive" role="alert">{{ error }}</p>

    <div v-else-if="previewIsValid" class="grid gap-3">
      <div class="grid gap-1">
        <div class="font-mono text-xs text-text-primary">{{ cron }}</div>
        <p class="text-muted-foreground">{{ description }}</p>
      </div>
      <div class="flex items-center justify-between gap-3">
        <div class="font-medium text-text-primary">{{ heading }}</div>
        <Button type="button" variant="outline" size="sm" @click="showUtc = !showUtc">
          {{ showUtc ? 'Show local' : 'Show UTC' }}
        </Button>
      </div>
      <p class="text-xs text-muted-foreground">Timezone: {{ showUtc ? 'UTC' : localTimeZone }}</p>
      <ul class="grid gap-1 text-muted-foreground">
        <li v-for="fire in nextFires" :key="fire">{{ formatFire(fire) }}</li>
      </ul>
    </div>
  </div>
</template>
