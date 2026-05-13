<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
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
import { useUnsavedChangesGuard } from '../composables/useUnsavedChangesGuard';

const props = defineProps<{
  note: MemoryFile;
}>();

const emit = defineEmits<{
  saved: [];
  cancel: [];
}>();

const store = useKnowledgeStore();
const guard = useUnsavedChangesGuard({ title: 'Discard knowledge note changes?' });
const draft = ref(props.note.content);
const saving = ref(false);
const error = ref<string | null>(null);
type ActiveDialog = 'conflict' | null;
const activeDialog = ref<ActiveDialog>(null);

function setActiveDialog(dialog: Exclude<ActiveDialog, null>, open: boolean): void {
  if (open) {
    activeDialog.value = dialog;
  } else if (activeDialog.value === dialog) {
    activeDialog.value = null;
  }
}

const conflictOpen = computed({
  get: () => activeDialog.value === 'conflict',
  set: (open: boolean) => {
    setActiveDialog('conflict', open);
  },
});
const hasUnsavedChanges = computed(() => draft.value !== props.note.content);

watch(() => props.note.content, (content) => {
  draft.value = content;
  guard.markClean();
});

let detachBeforeUnload: (() => void) | null = null;
onMounted(() => { detachBeforeUnload = guard.attachBeforeUnload(); });
onBeforeUnmount(() => { detachBeforeUnload?.(); });

function updateDraft(value: string): void {
  draft.value = value;
  guard.markDirty();
}

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
    guard.markClean();
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

function exitEditor(): void {
  draft.value = props.note.content;
  guard.markClean();
  emit('cancel');
}

async function requestCancel(): Promise<void> {
  error.value = null;
  if (!hasUnsavedChanges.value) {
    exitEditor();
    return;
  }
  if (await guard.confirmIfDirty()) exitEditor();
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
    guard.markClean();
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
      :model-value="draft"
      data-testid="inline-editor-content"
      class="min-h-[28rem] border-border-subtle bg-bg-surface font-mono text-body-sm text-text-primary"
      placeholder="Use [[WikiLinks]] to connect concepts"
      @update:model-value="(value: string | number) => updateDraft(String(value))"
    />

    <div class="flex justify-end gap-2">
      <Button type="button" variant="ghost" :disabled="saving" @click="requestCancel">Cancel</Button>
      <Button type="button" :disabled="saving" data-testid="inline-editor-save" @click="save">
        {{ saving ? 'Saving…' : 'Save' }}
      </Button>
    </div>

    <Dialog v-model:open="conflictOpen">
      <DialogContent class="border-border-card bg-bg-surface text-text-primary sm:max-w-md">
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
