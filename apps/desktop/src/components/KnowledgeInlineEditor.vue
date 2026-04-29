<script setup lang="ts">
import { ref, watch } from 'vue';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Textarea } from '@/components/ui/textarea';
import type { MemoryFile } from '../lib/workflow-types';
import { useKnowledgeStore } from '../stores/knowledge';

const props = defineProps<{
  note: MemoryFile;
}>();

const emit = defineEmits<{
  saved: [];
  cancel: [];
}>();

const store = useKnowledgeStore();
const draft = ref(props.note.content);
const saving = ref(false);
const error = ref<string | null>(null);
const conflictOpen = ref(false);

watch(() => props.note.content, (content) => {
  draft.value = content;
});

function isStaleEditError(errorValue: unknown): boolean {
  const message = errorValue instanceof Error ? errorValue.message : String(errorValue);
  return /checksum|stale|modified/i.test(message);
}

async function save(): Promise<void> {
  saving.value = true;
  error.value = null;
  try {
    await store.saveNoteContent({
      uri: props.note.uri,
      previousContent: props.note.content,
      content: draft.value,
      checksum: props.note.checksum,
    });
    emit('saved');
  } catch (errorValue) {
    if (isStaleEditError(errorValue)) {
      conflictOpen.value = true;
    } else {
      error.value = errorValue instanceof Error ? errorValue.message : String(errorValue);
    }
  } finally {
    saving.value = false;
  }
}

async function discardAndReload(): Promise<void> {
  await store.reloadNote(props.note.uri);
  conflictOpen.value = false;
}

async function forceOverwrite(): Promise<void> {
  saving.value = true;
  error.value = null;
  try {
    await store.forceSaveNoteContent(props.note.uri, draft.value);
    conflictOpen.value = false;
    emit('saved');
  } catch (errorValue) {
    error.value = errorValue instanceof Error ? errorValue.message : String(errorValue);
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <div class="grid gap-3">
    <Alert v-if="error" variant="destructive" class="border-border-subtle bg-bg-app">
      <AlertDescription>{{ error }}</AlertDescription>
    </Alert>

    <Textarea
      v-model="draft"
      data-testid="inline-editor-content"
      class="min-h-[28rem] border-border-subtle bg-bg-surface font-mono text-body-sm text-text-primary"
      placeholder="Use [[WikiLinks]] to connect concepts"
    />

    <div class="flex justify-end gap-2">
      <Button type="button" variant="ghost" @click="emit('cancel')">Cancel</Button>
      <Button type="button" :disabled="saving" data-testid="inline-editor-save" @click="save">
        {{ saving ? 'Saving…' : 'Save' }}
      </Button>
    </div>

    <Dialog v-model:open="conflictOpen">
      <DialogContent class="border-border-card bg-bg-surface text-text-primary">
        <DialogHeader>
          <DialogTitle>Edit conflict</DialogTitle>
          <DialogDescription>
            This note changed since it was opened. Reload or overwrite with your draft.
          </DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button type="button" variant="ghost" @click="discardAndReload">Discard mine and reload</Button>
          <Button type="button" :disabled="saving" @click="forceOverwrite">Force overwrite</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  </div>
</template>
