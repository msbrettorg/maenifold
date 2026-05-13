<script setup lang="ts">
import { Pin, X } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import { usePinnedSessionsStore } from '../stores/pinned-sessions';
import type { SavedSession } from '../lib/types';

const props = defineProps<{
  session: SavedSession;
  deleteTitle: string;
}>();

const emit = defineEmits<{
  delete: [session: SavedSession, event: Event];
}>();

const pinnedSessionsStore = usePinnedSessionsStore();
</script>

<template>
  <div class="flex items-center gap-1">
    <Button
      variant="ghost"
      size="icon"
      class="size-7 text-muted-foreground hover:bg-bg-surface-hover hover:text-text-primary"
      :title="pinnedSessionsStore.isPinned(session.id) ? 'Unpin session' : 'Pin session'"
      :aria-label="pinnedSessionsStore.isPinned(session.id) ? 'Unpin session' : 'Pin session'"
      :aria-pressed="pinnedSessionsStore.isPinned(session.id)"
      @click.stop="pinnedSessionsStore.togglePinnedSession(session.id)"
    >
      <Pin
        class="size-4"
        :class="pinnedSessionsStore.isPinned(session.id) ? 'fill-current text-brand' : ''"
      />
    </Button>
    <Button
      variant="ghost"
      size="icon"
      class="size-7 text-muted-foreground hover:bg-destructive/20 hover:text-destructive"
      :title="deleteTitle"
      @click="(event: Event) => emit('delete', session, event)"
    >
      <X class="size-4" />
    </Button>
  </div>
</template>
