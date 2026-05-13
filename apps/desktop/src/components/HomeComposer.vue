<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { Pencil, Send } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Textarea } from '@/components/ui/textarea';
import type { AppShellTab } from '../stores/app-shell';

const props = defineProps<{
  daemonOnline: boolean;
  isConnecting: boolean;
  submitPrompt: (prompt: string) => Promise<boolean>;
}>();

const emit = defineEmits<{
  'select-tab': [tab: AppShellTab];
  'open-tab': [tab: AppShellTab];
}>();

const composerInput = ref('');
const composerError = ref('');
const isSubmittingPrompt = ref(false);

const canSubmitPrompt = computed(() => (
  !isSubmittingPrompt.value
  && props.daemonOnline
  && !props.isConnecting
));

async function handleComposerSubmit() {
  const trimmedPrompt = composerInput.value.trim();
  if (!trimmedPrompt) return;
  if (!canSubmitPrompt.value) return;

  composerError.value = '';
  isSubmittingPrompt.value = true;

  try {
    const sent = await props.submitPrompt(trimmedPrompt);
    if (!sent) {
      composerError.value = 'Unable to send your prompt. Check the session connection and try again.';
      return;
    }
  } catch {
    composerError.value = 'Unable to send your prompt. Check the session connection and try again.';
    return;
  } finally {
    isSubmittingPrompt.value = false;
  }

  emit('select-tab', 'agents');
  emit('open-tab', 'agents');
  composerInput.value = '';
}

function handleComposerKeyDown(event: KeyboardEvent): void {
  if (event.isComposing || event.keyCode === 229) return;
  if (event.key === 'Enter' && !event.shiftKey) {
    event.preventDefault();
    handleComposerSubmit();
  }
}

watch(composerInput, () => {
  if (composerError.value) {
    composerError.value = '';
  }
});
</script>

<template>
  <Card class="w-full max-w-[800px] rounded-xl py-0 shadow-fluent-8 ring-0 border border-border-card bg-bg-elevated">
    <CardContent class="px-4 py-3">
      <form class="flex items-end gap-2" @submit.prevent="handleComposerSubmit">
        <Pencil class="mb-0.5 size-4 shrink-0 text-text-tertiary" :stroke-width="1.5" />
        <Textarea
          v-model="composerInput"
          placeholder="Ask me anything..."
          :aria-invalid="composerError ? 'true' : 'false'"
          aria-describedby="home-connection-status home-composer-hint home-composer-error"
          class="flex-1 min-h-0 max-h-32 field-sizing-content border-0 bg-transparent text-sm text-text-primary shadow-none outline-none ring-0 placeholder:text-text-tertiary focus-visible:border-transparent focus-visible:ring-0 px-0 py-0 rounded-none overflow-y-auto"
          @keydown="handleComposerKeyDown"
        />
        <Button
          type="submit"
          variant="ghost"
          size="icon"
          :disabled="!canSubmitPrompt"
          class="mb-0.5 flex size-9 items-center justify-center rounded-[4px] text-text-tertiary transition-colors hover:text-text-primary"
        >
          <Send class="size-4" :stroke-width="1.5" />
        </Button>
      </form>
      <p id="home-composer-hint" class="mt-1.5 text-[10px] text-text-tertiary">Shift+Enter for new line</p>
      <p
        v-if="composerError"
        id="home-composer-error"
        class="mt-2 text-xs text-danger"
      >
        {{ composerError }}
      </p>
    </CardContent>
  </Card>
</template>
