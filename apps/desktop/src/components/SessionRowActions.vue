<script setup lang="ts">
import { Pin, X } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from '@/components/ui/alert-dialog';
import { usePinnedSessionsStore } from '../stores/pinned-sessions';
import type { SavedSession } from '../lib/types';

const props = defineProps<{
  session: SavedSession;
  deleteTitle?: string;
}>();

const emit = defineEmits<{
  delete: [sessionId: string, event: Event];
}>();

const pinnedSessionsStore = usePinnedSessionsStore();

function isPinned(): boolean {
  return pinnedSessionsStore.isPinned(props.session.id);
}

function togglePinned(): void {
  pinnedSessionsStore.togglePinnedSession(props.session.id);
}
</script>

<template>
  <div class="flex items-center gap-1">
    <Button
      variant="ghost"
      size="icon"
      class="size-7 text-muted-foreground hover:bg-bg-surface-hover hover:text-text-primary"
      :title="isPinned() ? 'Unpin session' : 'Pin session'"
      :aria-label="isPinned() ? 'Unpin session' : 'Pin session'"
      :aria-pressed="isPinned()"
      @click.stop="togglePinned"
    >
      <Pin
        class="size-4"
        :class="isPinned() ? 'fill-current text-brand' : ''"
      />
    </Button>
    <AlertDialog>
      <AlertDialogTrigger as-child>
        <Button
          variant="ghost"
          size="icon"
          class="size-7 text-muted-foreground hover:bg-destructive/20 hover:text-destructive"
          :title="deleteTitle ?? 'Delete session'"
          :aria-label="deleteTitle ?? 'Delete session'"
          @click.stop
        >
          <X class="size-4" />
        </Button>
      </AlertDialogTrigger>
      <AlertDialogContent class="sm:max-w-[28rem]">
        <AlertDialogHeader>
          <AlertDialogTitle>{{ deleteTitle ?? 'Delete session' }}</AlertDialogTitle>
          <AlertDialogDescription>
            This will remove the local cache; the agent's server-side session is unaffected.
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <AlertDialogAction
            variant="destructive"
            @click="(event: Event) => emit('delete', props.session.id, event)"
          >
            Confirm
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  </div>
</template>
