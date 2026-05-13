<script setup lang="ts">
import { computed } from 'vue';
import { AlertCircle, Clock, XCircle } from 'lucide-vue-next';
import type { StopReason } from '../lib/types';

const props = defineProps<{
  stopReason?: StopReason | null;
}>();

const banner = computed(() => {
  switch (props.stopReason) {
    case 'max_tokens':
      return { text: 'Response truncated due to token limits.', icon: AlertCircle };
    case 'max_turn_requests':
      return { text: 'Turn limit reached.', icon: AlertCircle };
    case 'cancelled':
      return { text: 'Cancelled.', icon: XCircle };
    case 'refusal':
      return { text: 'The agent refused this request.', icon: AlertCircle };
    case 'tool_use':
      return { text: 'Awaiting tool result.', icon: Clock };
    case 'error':
      return { text: 'Error: see message.', icon: AlertCircle };
    case 'end_turn':
    case null:
    case undefined:
      return null;
    default: {
      // Compile-time exhaustiveness: a future StopReason value MUST add a case above.
      const _exhaustive: never = props.stopReason;
      void _exhaustive;
      return null;
    }
  }
});
</script>

<template>
  <span
    v-if="banner"
    class="inline-flex items-center gap-1 rounded-md bg-muted px-2 py-0.5 text-xs text-muted-foreground"
  >
    <component :is="banner.icon" class="size-3" aria-hidden="true" />
    <span>{{ banner.text }}</span>
  </span>
</template>
