<script setup lang="ts">
import { ChevronDown, FileText, Folder } from 'lucide-vue-next';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible';
import { Skeleton } from '@/components/ui/skeleton';
import { useKnowledgeStore, type KnowledgeFolderNode } from '../stores/knowledge';
import KnowledgeRowActions from './KnowledgeRowActions.vue';

const props = defineProps<{
  node: KnowledgeFolderNode;
  activeTabUri: string | null;
  filterActive?: boolean;
}>();

const store = useKnowledgeStore();

function handleOpenChange(nextOpen: boolean): void {
  void store.setFolderOpen(props.node.path, nextOpen);
}

function handleOpenNote(uri: string, event: MouseEvent): void {
  if (event.metaKey || event.ctrlKey) {
    store.openInNewTab(uri);
    return;
  }
  store.openNote(uri);
}
</script>

<template>
  <Collapsible :open="filterActive || node.open" class="rounded-md" @update:open="handleOpenChange">
    <CollapsibleTrigger as-child>
      <Button
        type="button"
        variant="ghost"
        class="h-8 w-full justify-start gap-2 rounded-md px-2 text-text-primary hover:bg-bg-surface-hover"
      >
        <ChevronDown
          class="size-4 shrink-0 text-text-secondary transition-transform"
          :class="filterActive || node.open ? 'rotate-0' : '-rotate-90'"
          aria-hidden="true"
        />
        <Folder class="size-4 shrink-0 text-brand" aria-hidden="true" />
        <span class="truncate text-body-sm font-medium">{{ node.name }}</span>
        <span class="ml-auto text-tiny text-text-tertiary">{{ node.fileCount }}</span>
      </Button>
    </CollapsibleTrigger>

    <CollapsibleContent class="pl-5">
      <div class="grid gap-1 py-1">
        <Alert v-if="node.error" variant="destructive" class="border-border-subtle bg-bg-sidebar">
          <AlertDescription>{{ node.error }}</AlertDescription>
        </Alert>

        <template v-else-if="node.loading">
          <Skeleton class="h-7 rounded-md" />
          <Skeleton class="h-7 rounded-md" />
        </template>

        <template v-else>
          <div
            v-for="file in node.files"
            :key="file.uri"
            class="flex h-8 items-center gap-1 rounded-md"
            :class="activeTabUri === file.uri ? 'bg-bg-surface text-text-primary' : 'text-text-secondary hover:bg-bg-surface-hover hover:text-text-primary'"
          >
            <Button type="button" variant="ghost" class="min-w-0 flex-1 justify-start gap-2 px-2" @click="handleOpenNote(file.uri, $event)">
              <FileText class="size-4 shrink-0" aria-hidden="true" />
              <span class="truncate text-body-sm font-medium">{{ file.name }}</span>
            </Button>
            <KnowledgeRowActions :file="file" />
          </div>

          <KnowledgeVaultNode
            v-for="child in node.children"
            :key="child.path"
            :node="child"
            :active-tab-uri="activeTabUri"
            :filter-active="filterActive"
          />
        </template>
      </div>
    </CollapsibleContent>
  </Collapsible>
</template>
