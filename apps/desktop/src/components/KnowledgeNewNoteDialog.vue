<script setup lang="ts">
import { computed, ref } from 'vue';
import { Plus } from 'lucide-vue-next';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { collectFolderPaths } from '../lib/knowledge-helpers';
import { useKnowledgeStore } from '../stores/knowledge';
import FolderPicker from './FolderPicker.vue';

const store = useKnowledgeStore();
const open = ref(false);
const title = ref('');
const folder = ref('');
const content = ref('');
const submitting = ref(false);
const error = ref<string | null>(null);

const availableFolders = computed(() => collectFolderPaths(store.folders));
const hasWikiLink = computed(() => /\[\[[^\]]+\]\]/.test(content.value));
const canSubmit = computed(() => title.value.trim().length > 0 && hasWikiLink.value && !submitting.value);

function resetForm(): void {
  title.value = '';
  folder.value = '';
  content.value = '';
  error.value = null;
}

async function submit(): Promise<void> {
  if (!canSubmit.value) return;
  submitting.value = true;
  error.value = null;
  try {
    await store.createNote({
      title: title.value.trim(),
      folder: folder.value.trim() || undefined,
      content: content.value,
    });
    open.value = false;
    resetForm();
  } catch (errorValue) {
    error.value = errorValue instanceof Error ? errorValue.message : String(errorValue);
  } finally {
    submitting.value = false;
  }
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogTrigger as-child>
      <Button type="button" size="sm" class="gap-2" data-testid="new-note-trigger">
        <Plus class="size-4" aria-hidden="true" />
        New note
      </Button>
    </DialogTrigger>
    <DialogContent class="border-border-card bg-bg-surface text-text-primary sm:max-w-2xl">
      <DialogHeader>
        <DialogTitle>Create knowledge note</DialogTitle>
        <DialogDescription>
          Use [[WikiLinks]] to connect concepts in the knowledge graph.
        </DialogDescription>
      </DialogHeader>

      <form class="grid gap-4" data-testid="new-note-form" @submit.prevent="submit">
        <Alert v-if="error" variant="destructive" class="border-border-subtle bg-bg-app">
          <AlertDescription>{{ error }}</AlertDescription>
        </Alert>

        <div class="grid gap-2">
          <Label for="new-note-title">Title</Label>
          <Input
            id="new-note-title"
            v-model="title"
            data-testid="new-note-title"
            class="border-border-subtle bg-bg-app"
            placeholder="Note title"
          />
        </div>

        <div class="grid gap-2">
          <Label for="new-note-folder">Folder</Label>
          <FolderPicker
            v-model="folder"
            :available-folders="availableFolders"
            data-testid="new-note-folder"
            placeholder="Existing folder or create new folder"
          />
        </div>

        <div class="grid gap-2">
          <Label for="new-note-content">Content</Label>
          <Textarea
            id="new-note-content"
            v-model="content"
            data-testid="new-note-content"
            class="min-h-48 border-border-subtle bg-bg-app"
            placeholder="Use [[WikiLinks]] to connect concepts"
          />
          <p class="text-tiny text-text-tertiary">
            Title and at least one [[WikiLink]] are required.
          </p>
        </div>

        <DialogFooter>
          <Button type="submit" :disabled="!canSubmit" data-testid="create-note-submit">
            {{ submitting ? 'Creating…' : 'Create note' }}
          </Button>
        </DialogFooter>
      </form>
    </DialogContent>
  </Dialog>
</template>
