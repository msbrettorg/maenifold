<script setup lang="ts">
import { ChevronDown, FileText, Folder } from 'lucide-vue-next';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible';
import { Skeleton } from '@/components/ui/skeleton';
import { useKnowledgeStore, type KnowledgeFolderNode } from '../stores/knowledge';

const props = defineProps<{
  node: KnowledgeFolderNode;
  activeUri: string | null;
}>();

const store = useKnowledgeStore();

function handleOpenChange(nextOpen: boolean) {
  void store.setFolderOpen(props.node.path, nextOpen);
}

function handleOpenNote(uri: string) {
  void store.openNote(uri);
}
</script>

<template>
  <Collapsible :open="node.open" class="rounded-[var(--radius-8)]" @update:open="handleOpenChange">
    <CollapsibleTrigger as-child>
      <Button
        type="button"
        variant="ghost"
        class="h-8 w-full justify-start gap-2 rounded-[var(--radius-8)] px-2 text-text-primary hover:bg-bg-surface-hover"
      >
        <ChevronDown
          class="size-4 shrink-0 text-text-secondary transition-transform"
          :class="node.open ? 'rotate-0' : '-rotate-90'"
          aria-hidden="true"
        />
        <Folder class="size-4 shrink-0 text-brand" aria-hidden="true" />
        <span class="truncate text-[var(--text-body-sm)] font-medium">{{ node.name }}</span>
        <span class="ml-auto text-[var(--text-tiny)] text-text-tertiary">{{ node.fileCount }}</span>
      </Button>
    </CollapsibleTrigger>

    <CollapsibleContent class="pl-5">
      <div class="grid gap-1 py-1">
        <Alert v-if="node.error" variant="destructive" class="border-border-subtle bg-bg-sidebar">
          <AlertDescription>{{ node.error }}</AlertDescription>
        </Alert>

        <template v-else-if="node.loading">
          <Skeleton class="h-7 rounded-[var(--radius-8)]" />
          <Skeleton class="h-7 rounded-[var(--radius-8)]" />
        </template>

        <template v-else>
          <Button
            v-for="file in node.files"
            :key="file.uri"
            type="button"
            variant="ghost"
            class="h-8 justify-start gap-2 rounded-[var(--radius-8)] px-2"
            :class="activeUri === file.uri ? 'bg-bg-surface text-text-primary' : 'text-text-secondary hover:bg-bg-surface-hover hover:text-text-primary'"
            @click="handleOpenNote(file.uri)"
          >
            <FileText class="size-4 shrink-0" aria-hidden="true" />
            <span class="truncate text-[var(--text-body-sm)] font-medium">{{ file.name }}</span>
          </Button>

          <KnowledgeSidebarNode
            v-for="child in node.children"
            :key="child.path"
            :node="child"
            :active-uri="activeUri"
          />
        </template>
      </div>
    </CollapsibleContent>
  </Collapsible>
</template>
