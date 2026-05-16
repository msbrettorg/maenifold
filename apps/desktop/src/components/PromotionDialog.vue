<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { toast } from 'vue-sonner';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { ScrollArea } from '@/components/ui/scroll-area';
import type { ViewNode } from '../lib/pipeline-types-view';
import type { MemoryPromotionSource, NormalizedMemoryDocument, NormalizedMemorySearchItem } from '../composables/useMemoryPromotion';
import { buildPromotionPreview, hasWikiLinkConnection, isStaleMemoryEditError, useMemoryPromotion } from '../composables/useMemoryPromotion';
import { useUnsavedChangesGuard } from '../composables/useUnsavedChangesGuard';
import PromotionDialogConfirm from './PromotionDialogConfirm.vue';
import PromotionDialogEditor from './PromotionDialogEditor.vue';
import PromotionDialogPreview from './PromotionDialogPreview.vue';

type PromotionAction = 'create' | 'append' | 'update';
type PromotionStep = 'preview' | 'editor' | 'confirm';

const props = defineProps<{ node: ViewNode; pipelineId: string }>();
const emit = defineEmits<{ (event: 'close'): void; (event: 'busy-change', busy: boolean): void }>();

const promotion = useMemoryPromotion();
const guard = useUnsavedChangesGuard({ title: 'Discard promotion changes?' });
const step = ref<PromotionStep>('preview');
const searchQuery = ref(createDefaultTitle(props.node.nodeId));
const title = ref(createDefaultTitle(props.node.nodeId)), folder = ref(''), tagsInput = ref(''), wikiLinkInput = ref('');
const formError = ref(''), infoMessage = ref('');
const searchResults = ref<NormalizedMemorySearchItem[]>([]);
const selectedAction = ref<PromotionAction | null>(null);
const selectedResult = ref<NormalizedMemorySearchItem | null>(null);
const selectedMemory = ref<NormalizedMemoryDocument | null>(null);
const loadingMemory = ref(false);
const editorRef = ref<{ focusWikiLinkInput?: () => void } | null>(null);

const source = computed<MemoryPromotionSource>(() => ({ pipelineId: props.pipelineId, nodeId: props.node.nodeId, output: props.node.output ?? '' }));
const wikiLinkError = computed(() => hasWikiLinkConnection(wikiLinkInput.value) ? '' : 'Add at least one connection before saving.');
const tags = computed(() => tagsInput.value.split(/[\n,]/).map((tag) => tag.trim()).filter(Boolean));
const canSubmitUpdate = computed(() => Boolean(selectedMemory.value?.checksum));
const updateDisabledMessage = computed(() => {
  if (!selectedMemory.value) return 'Open a note first.';
  if (!selectedMemory.value.checksum) return 'This note could not be updated safely because no checksum was returned.';
  return '';
});
const nextPreview = computed(() => {
  if (!hasWikiLinkConnection(wikiLinkInput.value)) {
    return 'Add at least one connection to preview the saved content.';
  }
  return buildPromotionPreview(source.value, wikiLinkInput.value, '2026-01-01T00:00:00.000Z')
    .replace('2026-01-01T00:00:00.000Z', '<ISO timestamp>');
});
const submitLabel = computed(() => {
  if (promotion.writing.value) {
    return selectedAction.value === 'append'
      ? 'Appending…'
      : selectedAction.value === 'update'
        ? 'Updating…'
        : 'Writing…';
  }
  return selectedAction.value === 'append'
    ? 'Append'
    : selectedAction.value === 'update'
      ? 'Update'
      : 'Write';
});

watch(() => promotion.busy.value, (busy) => emit('busy-change', busy), { immediate: true });
let detachBeforeUnload: (() => void) | null = null;
onMounted(() => {
  detachBeforeUnload = guard.attachBeforeUnload();
  void runSearch();
});
onBeforeUnmount(() => {
  detachBeforeUnload?.();
});

function createDefaultTitle(nodeId: string): string {
  const normalized = nodeId.replace(/[_-]+/g, ' ').trim();
  if (!normalized) return nodeId;
  return normalized
    .split(/\s+/)
    .map((segment) => segment.charAt(0).toUpperCase() + segment.slice(1))
    .join(' ');
}

async function runSearch(): Promise<void> {
  formError.value = '';
  infoMessage.value = '';
  selectedAction.value = null;
  selectedResult.value = null;
  selectedMemory.value = null;
  step.value = 'preview';
  promotion.resetResultState();

  const normalizedQuery = searchQuery.value.trim();
  if (!normalizedQuery) {
    formError.value = 'Enter a title or topic to search.';
    searchResults.value = [];
    return;
  }

  try {
    const result = await promotion.searchMemories(normalizedQuery);
    searchResults.value = result.results;
    if (result.results.length === 0) {
      infoMessage.value = 'No existing notes found. You can create a new note below.';
      selectedAction.value = 'create';
    }
  } catch {
    searchResults.value = [];
  }
}

async function selectAction(action: PromotionAction, result?: NormalizedMemorySearchItem): Promise<void> {
  formError.value = '';
  infoMessage.value = '';
  promotion.resetResultState();
  selectedAction.value = action;
  selectedResult.value = result ?? null;
  selectedMemory.value = null;

  if (!result || action === 'create') {
    step.value = 'editor';
    return;
  }

  loadingMemory.value = true;
  try {
    selectedMemory.value = await promotion.readMemory(result.uri, action === 'update');
    step.value = 'editor';
  } catch {
    if (!formError.value && promotion.error.value) {
      formError.value = promotion.error.value;
    }
  } finally {
    loadingMemory.value = false;
  }
}

async function retryCurrentAction(): Promise<void> {
  if (selectedAction.value === 'append' && selectedResult.value) {
    await selectAction('append', selectedResult.value);
    return;
  }
  if (selectedAction.value === 'update' && selectedResult.value) {
    await selectAction('update', selectedResult.value);
    return;
  }
  await runSearch();
}

function validateCurrentAction(): boolean {
  formError.value = '';
  if (!selectedAction.value) {
    formError.value = 'Choose how you want to save this note.';
    return false;
  }
  if (!hasWikiLinkConnection(wikiLinkInput.value)) {
    formError.value = wikiLinkError.value;
    toast.error('Add at least one connection before saving');
    editorRef.value?.focusWikiLinkInput?.();
    return false;
  }
  if (selectedAction.value === 'append' && !selectedResult.value) {
    formError.value = 'Choose a note to append to.';
    return false;
  }
  if (selectedAction.value === 'update') {
    if (!selectedResult.value || !selectedMemory.value) {
      formError.value = 'Choose a note to update.';
      return false;
    }
    if (!canSubmitUpdate.value) {
      formError.value = updateDisabledMessage.value;
      return false;
    }
  }
  if (selectedAction.value === 'create' && !title.value.trim()) {
    formError.value = 'Title is required.';
    return false;
  }
  return true;
}

function continueToConfirm(): void {
  if (validateCurrentAction()) {
    step.value = 'confirm';
  }
}

function updateTitle(value: string): void { title.value = value; guard.markDirty(); }
function updateFolder(value: string): void { folder.value = value; guard.markDirty(); }
function updateTagsInput(value: string): void { tagsInput.value = value; guard.markDirty(); }
function updateWikiLinkInput(value: string): void { wikiLinkInput.value = value; guard.markDirty(); }

async function backToPreview(): Promise<void> {
  if (!(await guard.confirmIfDirty())) return;
  step.value = 'preview';
}

async function handleSubmit(): Promise<void> {
  promotion.resetResultState();
  if (!validateCurrentAction()) return;

  try {
    if (selectedAction.value === 'append' && selectedResult.value) {
      await promotion.appendMemory(source.value, {
        identifier: selectedResult.value.uri,
        wikiLinkInput: wikiLinkInput.value,
      });
      guard.markClean();
      return;
    }

    if (selectedAction.value === 'update' && selectedResult.value && selectedMemory.value) {
      try {
        await promotion.updateMemory(source.value, {
          identifier: selectedResult.value.uri,
          wikiLinkInput: wikiLinkInput.value,
          checksum: selectedMemory.value.checksum,
        });
      } catch (errorValue) {
        if (isStaleMemoryEditError(errorValue)) {
          selectedMemory.value = await promotion.readMemory(selectedResult.value.uri, true);
          step.value = 'editor';
        }
        throw errorValue;
      }
      guard.markClean();
      return;
    }

    await promotion.writeMemory(source.value, {
      title: title.value.trim(),
      wikiLinkInput: wikiLinkInput.value,
      folder: folder.value,
      tags: tags.value,
    });
    guard.markClean();
  } catch {
    if (!formError.value && promotion.error.value) {
      formError.value = promotion.error.value;
    }
  }
}

async function closeDialog(): Promise<void> {
  if (!(await guard.confirmIfDirty())) return;
  emit('close');
}
</script>

<template>
  <Dialog :open="true" @update:open="(open) => !open && closeDialog()">
    <DialogContent class="w-[min(64rem,100%)] max-h-[min(92vh,60rem)] gap-0 bg-surface-container p-0 sm:max-w-[64rem]" :show-close-button="false">
      <DialogHeader class="flex flex-row justify-between items-start gap-3 p-4 max-[720px]:flex-col">
        <div>
          <DialogTitle id="promotion-dialog-title" class="font-display text-title font-medium text-text-primary">Promote to memory</DialogTitle>
          <DialogDescription class="text-body-sm text-text-secondary">Save this finished node output as a reusable note or add it to one you already have.</DialogDescription>
        </div>
        <Button type="button" variant="ghost" class="rounded-[4px]" :disabled="promotion.busy.value" @click="closeDialog">Close</Button>
      </DialogHeader>
      <ScrollArea class="max-h-[min(calc(92vh-6rem),54rem)]">
        <section v-if="promotion.writtenUri.value" class="grid gap-4 px-4 pb-4 pt-2" aria-live="polite">
          <h4 class="m-0 font-display text-base font-medium text-text-primary">Saved</h4>
          <p class="text-text-secondary">This output is now stored in memory.</p>
          <code class="m-0 rounded-[4px] bg-surface p-3 font-mono whitespace-pre-wrap break-all">{{ promotion.writtenUri.value }}</code>
          <div class="flex flex-wrap items-center gap-3 [&>:first-child:not(:only-child)]:mr-auto">
            <Button type="button" class="rounded-[4px]" @click="closeDialog">Done</Button>
          </div>
        </section>
        <PromotionDialogPreview v-else-if="step === 'preview'" :search-query="searchQuery" :search-results="searchResults" :searching="promotion.searching.value" :writing="promotion.writing.value" :info-message="infoMessage" :form-error="formError" :promotion-error="promotion.error.value" :busy="promotion.busy.value" @update:search-query="searchQuery = $event" @search="runSearch" @select-action="selectAction" @close="closeDialog" />
        <PromotionDialogEditor ref="editorRef" v-else-if="step === 'editor' && selectedAction" :selected-action="selectedAction" :selected-result="selectedResult" :selected-memory="selectedMemory" :loading-memory="loadingMemory" :title="title" :folder="folder" :tags-input="tagsInput" :wiki-link-input="wikiLinkInput" :wiki-link-error="wikiLinkError" :next-preview="nextPreview" :form-error="formError" :update-disabled-message="updateDisabledMessage" :busy="promotion.busy.value" @update:title="updateTitle" @update:folder="updateFolder" @update:tags-input="updateTagsInput" @update:wiki-link-input="updateWikiLinkInput" @back="backToPreview" @continue="continueToConfirm" @retry="retryCurrentAction" @close="closeDialog" />
        <PromotionDialogConfirm v-else :selected-action="selectedAction" :selected-result="selectedResult" :selected-memory="selectedMemory" :title="title" :folder="folder" :tags="tags" :next-preview="nextPreview" :form-error="formError" :submit-label="submitLabel" :busy="promotion.busy.value" :can-submit-update="canSubmitUpdate" :update-disabled-message="updateDisabledMessage" @back="step = 'editor'" @submit="handleSubmit" @retry="retryCurrentAction" @close="closeDialog" />
      </ScrollArea>
    </DialogContent>
  </Dialog>
</template>
