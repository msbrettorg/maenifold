<script setup lang="ts">
import { computed, ref } from 'vue';
import { MoreHorizontal } from 'lucide-vue-next';
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
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import type { KnowledgeFileEntry } from '../stores/knowledge';
import { useKnowledgeStore } from '../stores/knowledge';

const props = defineProps<{
  file: KnowledgeFileEntry;
}>();

const store = useKnowledgeStore();
const deleteOpen = ref(false);
const renameOpen = ref(false);
const moveOpen = ref(false);
const renameValue = ref(props.file.name);
const moveFolder = ref('');
const error = ref<string | null>(null);
const busy = ref(false);

const folderPath = computed(() => {
  const index = props.file.path.lastIndexOf('/');
  return index > -1 ? props.file.path.slice(0, index) : '';
});

function joinPath(folder: string, name: string): string {
  const normalizedFolder = folder.trim().replace(/^\/+|\/+$/g, '');
  const normalizedName = name.trim().replace(/^\/+/, '');
  if (!normalizedFolder) return normalizedName;
  return `${normalizedFolder}/${normalizedName}`;
}

function openRename(): void {
  renameValue.value = props.file.name;
  error.value = null;
  renameOpen.value = true;
}

function openMove(): void {
  moveFolder.value = folderPath.value;
  error.value = null;
  moveOpen.value = true;
}

async function deleteNote(): Promise<void> {
  busy.value = true;
  error.value = null;
  try {
    await store.deleteNote(props.file.uri);
    deleteOpen.value = false;
  } catch (errorValue) {
    error.value = errorValue instanceof Error ? errorValue.message : String(errorValue);
  } finally {
    busy.value = false;
  }
}

async function renameNote(): Promise<void> {
  const nextName = renameValue.value.trim();
  if (!nextName) return;
  busy.value = true;
  error.value = null;
  try {
    await store.moveNote(props.file.uri, joinPath(folderPath.value, nextName));
    renameOpen.value = false;
  } catch (errorValue) {
    error.value = errorValue instanceof Error ? errorValue.message : String(errorValue);
  } finally {
    busy.value = false;
  }
}

async function moveNote(): Promise<void> {
  busy.value = true;
  error.value = null;
  try {
    await store.moveNote(props.file.uri, joinPath(moveFolder.value, props.file.name));
    moveOpen.value = false;
  } catch (errorValue) {
    error.value = errorValue instanceof Error ? errorValue.message : String(errorValue);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <div class="shrink-0" @click.stop>
    <DropdownMenu>
      <DropdownMenuTrigger as-child>
        <Button type="button" size="icon-sm" variant="ghost" aria-label="Note actions">
          <MoreHorizontal class="size-4" aria-hidden="true" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" class="border-border-card bg-bg-surface text-text-primary">
        <DropdownMenuItem data-testid="row-action-rename" @select="openRename">Rename</DropdownMenuItem>
        <DropdownMenuItem data-testid="row-action-move" @select="openMove">Move</DropdownMenuItem>
        <DropdownMenuItem class="text-danger" data-testid="row-action-delete" @select="deleteOpen = true">
          Delete
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>

    <Dialog v-model:open="deleteOpen">
      <DialogContent class="border-border-card bg-bg-surface text-text-primary">
        <DialogHeader>
          <DialogTitle>Delete note</DialogTitle>
          <DialogDescription>
            This will permanently delete {{ file.name }}. This cannot be undone.
          </DialogDescription>
        </DialogHeader>
        <Alert v-if="error" variant="destructive" class="border-border-subtle bg-bg-app">
          <AlertDescription>{{ error }}</AlertDescription>
        </Alert>
        <DialogFooter>
          <Button type="button" variant="ghost" @click="deleteOpen = false">Cancel</Button>
          <Button type="button" variant="destructive" :disabled="busy" data-testid="confirm-delete-note" @click="deleteNote">
            Delete
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>

    <Dialog v-model:open="renameOpen">
      <DialogContent class="border-border-card bg-bg-surface text-text-primary">
        <DialogHeader>
          <DialogTitle>Rename note</DialogTitle>
          <DialogDescription>Rename within {{ folderPath || 'the vault root' }}.</DialogDescription>
        </DialogHeader>
        <div class="grid gap-2">
          <Label for="rename-note-input">File name</Label>
          <Input id="rename-note-input" v-model="renameValue" data-testid="rename-note-input" class="border-border-subtle bg-bg-app" />
        </div>
        <Alert v-if="error" variant="destructive" class="border-border-subtle bg-bg-app">
          <AlertDescription>{{ error }}</AlertDescription>
        </Alert>
        <DialogFooter>
          <Button type="button" variant="ghost" @click="renameOpen = false">Cancel</Button>
          <Button type="button" :disabled="busy || !renameValue.trim()" data-testid="confirm-rename-note" @click="renameNote">
            Rename
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>

    <Dialog v-model:open="moveOpen">
      <DialogContent class="border-border-card bg-bg-surface text-text-primary">
        <DialogHeader>
          <DialogTitle>Move note</DialogTitle>
          <DialogDescription>Choose the destination folder for {{ file.name }}.</DialogDescription>
        </DialogHeader>
        <div class="grid gap-2">
          <Label for="move-note-folder">Folder</Label>
          <Input id="move-note-folder" v-model="moveFolder" data-testid="move-note-folder" class="border-border-subtle bg-bg-app" />
        </div>
        <Alert v-if="error" variant="destructive" class="border-border-subtle bg-bg-app">
          <AlertDescription>{{ error }}</AlertDescription>
        </Alert>
        <DialogFooter>
          <Button type="button" variant="ghost" @click="moveOpen = false">Cancel</Button>
          <Button type="button" :disabled="busy" data-testid="confirm-move-note" @click="moveNote">
            Move
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  </div>
</template>
