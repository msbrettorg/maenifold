<script setup lang="ts">
import { Pin, Square, X } from 'lucide-vue-next';
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
import type { SamplingHistoryEntry, SavedSession } from '../lib/types';

const props = defineProps<{
  session: SavedSession | SamplingHistoryEntry;
  deleteTitle?: string;
  source?: 'agent' | 'sampling';
  canStop?: boolean;
}>();

const emit = defineEmits<{
  delete: [sessionId: string, event: Event];
  stop: [samplingRequestId: string, event: Event];
}>();

const pinnedSessionsStore = usePinnedSessionsStore();

function isSamplingSession(): boolean {
  return props.source === 'sampling';
}

function actionId(): string {
  return isSamplingSession()
    ? (props.session as SamplingHistoryEntry).samplingRequestId
    : (props.session as SavedSession).id;
}

function isPinned(): boolean {
  return !isSamplingSession() && pinnedSessionsStore.isPinned((props.session as SavedSession).id);
}

function togglePinned(): void {
  if (!isSamplingSession()) pinnedSessionsStore.togglePinnedSession((props.session as SavedSession).id);
}
</script>

<template>
  <div class="flex items-center gap-1">
    <Button
      v-if="!isSamplingSession()"
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
      <AlertDialogTrigger v-if="!isSamplingSession()" as-child>
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
            @click="(event: Event) => emit('delete', actionId(), event)"
          >
            Confirm
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
    <Button
      v-if="isSamplingSession() && canStop"
      variant="ghost"
      size="icon"
      class="size-7 text-muted-foreground hover:bg-destructive/20 hover:text-destructive"
      title="Stop sampling session"
      aria-label="Stop sampling session"
      @click.stop="(event: Event) => emit('stop', actionId(), event)"
    >
      <Square class="size-3.5 fill-current" />
    </Button>
  </div>
</template>
