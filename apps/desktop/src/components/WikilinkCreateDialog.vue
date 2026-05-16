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
import { Label } from '@/components/ui/label';
import { useMemoryApi } from '../composables/useMemoryApi';
import { sanitizeConceptForWikilinkQuery } from '../lib/wikilink-query-sanitizer';
import FolderPicker from './FolderPicker.vue';

const open = defineModel<boolean>('open', { required: true });
const props = defineProps<{
  concept: string;
  availableFolders: string[];
}>();
const emit = defineEmits<{
  created: [uri: string];
  cancel: [];
}>();

const folder = ref('');
const submitting = ref(false);
const error = ref<string | null>(null);
const memoryApi = useMemoryApi();

watch(open, (isOpen) => {
  if (!isOpen) return;
  folder.value = '';
  error.value = null;
});

function cancel(): void {
  open.value = false;
  emit('cancel');
}

async function createNote(): Promise<void> {
  if (submitting.value) return;
  submitting.value = true;
  error.value = null;
  try {
    const uri = await memoryApi.writeMemory({
      title: props.concept,
      content: `[[${sanitizeConceptForWikilinkQuery(props.concept)}]]`,
      folder: folder.value.trim() || undefined,
    });
    open.value = false;
    emit('created', uri);
  } catch (errorValue) {
    error.value = errorValue instanceof Error ? errorValue.message : String(errorValue);
  } finally {
    submitting.value = false;
  }
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent class="border-border-card bg-bg-surface text-text-primary sm:max-w-md">
      <DialogHeader>
        <DialogTitle>Create note for “{{ concept }}”?</DialogTitle>
        <DialogDescription>
          This will create a new note titled “{{ concept }}” containing a <code>[[{{ concept }}]]</code> link.
        </DialogDescription>
      </DialogHeader>

      <form class="grid gap-4" @submit.prevent="createNote">
        <Alert v-if="error" variant="destructive" class="border-border-subtle bg-bg-app">
          <AlertDescription>{{ error }}</AlertDescription>
        </Alert>

        <div class="grid gap-2">
          <Label for="wikilink-create-folder">Folder</Label>
          <FolderPicker
            id="wikilink-create-folder"
            v-model="folder"
            :available-folders="availableFolders"
            placeholder="Root folder"
          />
        </div>

        <DialogFooter>
          <Button
            type="button"
            variant="outline"
            data-testid="wikilink-create-cancel"
            @click="cancel"
          >
            Cancel
          </Button>
          <Button
            type="button"
            :disabled="submitting"
            data-testid="wikilink-create-submit"
            @click="createNote"
          >
            {{ submitting ? 'Creating…' : 'Create' }}
          </Button>
        </DialogFooter>
      </form>
    </DialogContent>
  </Dialog>
</template>
