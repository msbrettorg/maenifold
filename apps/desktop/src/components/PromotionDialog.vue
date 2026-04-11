<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import type { ViewNode } from '../lib/pipeline-types';
import type {
  MemoryPromotionSource,
  NormalizedMemoryDocument,
  NormalizedMemorySearchItem,
} from '../composables/useMemoryPromotion';
import {
  buildPromotionPreview,
  hasWikiLinkConnection,
  isStaleMemoryEditError,
  useMemoryPromotion,
} from '../composables/useMemoryPromotion';

type PromotionAction = 'create' | 'append' | 'update';

const props = defineProps<{
  node: ViewNode;
  pipelineId: string;
}>();

const emit = defineEmits<{
  (event: 'close'): void;
  (event: 'busy-change', busy: boolean): void;
}>();

const promotion = useMemoryPromotion();
const searchQuery = ref(createDefaultTitle(props.node.nodeId));
const title = ref(createDefaultTitle(props.node.nodeId));
const folder = ref('');
const tagsInput = ref('');
const wikiLinkInput = ref('');
const formError = ref('');
const infoMessage = ref('');
const searchResults = ref<NormalizedMemorySearchItem[]>([]);
const selectedAction = ref<PromotionAction | null>(null);
const selectedResult = ref<NormalizedMemorySearchItem | null>(null);
const selectedMemory = ref<NormalizedMemoryDocument | null>(null);
const loadingMemory = ref(false);

const source = computed<MemoryPromotionSource>(() => ({
  pipelineId: props.pipelineId,
  nodeId: props.node.nodeId,
  output: props.node.output ?? '',
}));
const wikiLinkError = computed(() => hasWikiLinkConnection(wikiLinkInput.value) ? '' : 'Add at least one connection before saving.');
const tags = computed(() =>
  tagsInput.value
    .split(/[\n,]/)
    .map((tag) => tag.trim())
    .filter(Boolean),
);
const canSubmitUpdate = computed(() => Boolean(selectedMemory.value?.checksum));
const updateDisabledMessage = computed(() => {
  if (!selectedMemory.value) {
    return 'Open a note first.';
  }

  if (!selectedMemory.value.checksum) {
    return 'This note could not be updated safely because no checksum was returned.';
  }

  return '';
});
const nextPreview = computed(() => {
  if (!hasWikiLinkConnection(wikiLinkInput.value)) {
    return 'Add at least one connection to preview the saved content.';
  }

  return buildPromotionPreview(source.value, wikiLinkInput.value, '2026-01-01T00:00:00.000Z')
    .replace('2026-01-01T00:00:00.000Z', '<ISO timestamp>');
});

watch(
  () => promotion.busy.value,
  (busy) => emit('busy-change', busy),
  { immediate: true },
);

onMounted(() => {
  void runSearch();
});

function createDefaultTitle(nodeId: string): string {
  const normalized = nodeId.replace(/[_-]+/g, ' ').trim();
  if (!normalized) {
    return nodeId;
  }

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
    return;
  }

  loadingMemory.value = true;
  try {
    selectedMemory.value = await promotion.readMemory(result.uri, action === 'update');
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

async function handleSubmit(): Promise<void> {
  formError.value = '';
  promotion.resetResultState();

  if (!hasWikiLinkConnection(wikiLinkInput.value)) {
    formError.value = wikiLinkError.value;
    return;
  }

  try {
    if (selectedAction.value === 'append') {
      if (!selectedResult.value) {
        formError.value = 'Choose a note to append to.';
        return;
      }

      await promotion.appendMemory(source.value, {
        identifier: selectedResult.value.uri,
        wikiLinkInput: wikiLinkInput.value,
      });
      return;
    }

    if (selectedAction.value === 'update') {
      if (!selectedResult.value || !selectedMemory.value) {
        formError.value = 'Choose a note to update.';
        return;
      }

      if (!canSubmitUpdate.value) {
        formError.value = updateDisabledMessage.value;
        return;
      }

      try {
        await promotion.updateMemory(source.value, {
          identifier: selectedResult.value.uri,
          wikiLinkInput: wikiLinkInput.value,
          checksum: selectedMemory.value.checksum,
        });
      } catch (errorValue) {
        if (isStaleMemoryEditError(errorValue)) {
          selectedMemory.value = await promotion.readMemory(selectedResult.value.uri, true);
        }
        throw errorValue;
      }
      return;
    }

    const normalizedTitle = title.value.trim();
    if (!normalizedTitle) {
      formError.value = 'Title is required.';
      return;
    }

    await promotion.writeMemory(source.value, {
      title: normalizedTitle,
      wikiLinkInput: wikiLinkInput.value,
      folder: folder.value,
      tags: tags.value,
    });
  } catch {
    if (!formError.value && promotion.error.value) {
      formError.value = promotion.error.value;
    }
  }
}

function closeDialog(): void {
  emit('close');
}

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
</script>

<template>
  <div class="promotion-overlay" @click.self="closeDialog">
    <div class="promotion-dialog panel" role="dialog" aria-modal="true" aria-labelledby="promotion-dialog-title">
      <div class="dialog-header">
        <div>
          <h3 id="promotion-dialog-title">Promote to Memory</h3>
          <p>Save this finished node output as a reusable note or add it to one you already have.</p>
        </div>
        <button type="button" class="btn btn-ghost" :disabled="promotion.busy.value" @click="closeDialog">Close</button>
      </div>

      <section v-if="promotion.writtenUri.value" class="dialog-section success-state" aria-live="polite">
        <h4>Saved</h4>
        <p>This output is now stored in memory.</p>
        <code class="mono">{{ promotion.writtenUri.value }}</code>
        <div class="dialog-actions">
          <button type="button" class="btn btn-primary" @click="closeDialog">Done</button>
        </div>
      </section>

      <template v-else>
        <section class="dialog-section">
          <div class="search-row">
            <label class="field-block search-field">
              <span>Search existing notes</span>
              <input v-model.trim="searchQuery" class="field-input" type="text" placeholder="Search by title or topic" />
            </label>
            <button type="button" class="btn btn-ghost search-button" :disabled="promotion.searching.value || promotion.writing.value" @click="runSearch">
              {{ promotion.searching.value ? 'Searching…' : 'Search again' }}
            </button>
          </div>
          <p v-if="infoMessage" class="info-message">{{ infoMessage }}</p>
          <p v-if="promotion.error.value && !formError" class="form-error" role="alert">{{ promotion.error.value }}</p>

          <div v-if="searchResults.length > 0" class="result-list">
            <article v-for="result in searchResults" :key="result.uri" class="result-card">
              <div class="result-card__body">
                <strong>{{ result.title }}</strong>
                <code class="mono">{{ result.uri }}</code>
                <p v-if="result.snippet">{{ result.snippet }}</p>
                <span class="result-score">Score {{ result.score.toFixed(2) }}</span>
              </div>
              <div class="result-card__actions">
                <button type="button" class="btn btn-ghost" :disabled="promotion.busy.value" @click="selectAction('append', result)">Append to this</button>
                <button type="button" class="btn btn-ghost" :disabled="promotion.busy.value" @click="selectAction('update', result)">Update this</button>
              </div>
            </article>
          </div>

          <div class="dialog-actions dialog-actions--start">
            <button type="button" class="btn btn-primary" :disabled="promotion.busy.value" @click="selectAction('create')">Create new</button>
          </div>
        </section>

        <section v-if="selectedAction" class="dialog-section action-section">
          <div class="section-header">
            <div>
              <h4>
                {{
                  selectedAction === 'append'
                    ? 'Append to existing note'
                    : selectedAction === 'update'
                      ? 'Update existing note'
                      : 'Create new note'
                }}
              </h4>
              <p>
                {{
                  selectedAction === 'append'
                    ? 'We will add a new promoted block to the selected note.'
                    : selectedAction === 'update'
                      ? 'We will ask the server to replace the note’s Promoted Content section when we can do it safely.'
                      : 'We will create a new note with this promoted block.'
                }}
              </p>
            </div>
            <button type="button" class="btn btn-ghost" :disabled="promotion.busy.value" @click="selectedAction = null">Back</button>
          </div>

          <div v-if="selectedAction === 'create'" class="form-grid">
            <label class="field-block">
              <span>Title</span>
              <input v-model.trim="title" class="field-input" type="text" placeholder="Note title" />
            </label>
            <label class="field-block">
              <span>Folder</span>
              <input v-model.trim="folder" class="field-input" type="text" placeholder="Optional folder" />
            </label>
            <label class="field-block field-wide">
              <span>Tags</span>
              <input v-model.trim="tagsInput" class="field-input" type="text" placeholder="Optional tags, separated by commas" />
            </label>
          </div>

          <label class="field-block">
            <span>Connections</span>
            <textarea
              v-model="wikiLinkInput"
              class="field-textarea"
              rows="3"
              placeholder="Add one or more topics, for example: semantic zoom, pipeline-ui"
            />
            <small class="field-help">Use plain topics or [[WikiLinks]]. We will format them for you.</small>
          </label>
          <p v-if="wikiLinkError" class="form-error" role="alert">{{ wikiLinkError }}</p>

          <div v-if="selectedAction !== 'create'" class="memory-preview">
            <span>Current note</span>
            <pre v-if="selectedMemory">{{ selectedMemory.content }}</pre>
            <pre v-else-if="loadingMemory">Loading note…</pre>
            <pre v-else>Select a note to continue.</pre>
          </div>

          <p v-if="selectedAction === 'update' && updateDisabledMessage" class="info-message">{{ updateDisabledMessage }}</p>

          <div class="memory-preview">
            <span>What will be saved</span>
            <pre>{{ nextPreview }}</pre>
          </div>

          <p v-if="formError" class="form-error" role="alert">{{ formError }}</p>

          <div class="dialog-actions">
            <button type="button" class="btn btn-ghost" :disabled="promotion.busy.value" @click="closeDialog">Cancel</button>
            <button type="button" class="btn btn-ghost" :disabled="promotion.busy.value" @click="retryCurrentAction">Retry</button>
            <button
              type="button"
              class="btn btn-primary"
              :disabled="promotion.writing.value || (selectedAction === 'update' && !canSubmitUpdate)"
              @click="handleSubmit"
            >
              {{ submitLabel }}
            </button>
          </div>
        </section>
      </template>
    </div>
  </div>
</template>

<style scoped>
.promotion-overlay {
  position: fixed;
  inset: 0;
  z-index: 1000;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 1.5rem;
  background: var(--bg-overlay);
}

.promotion-dialog {
  width: min(64rem, 100%);
  max-height: min(92vh, 60rem);
  overflow: auto;
  display: grid;
  gap: 1rem;
}

.dialog-header,
.section-header,
.search-row,
.dialog-actions,
.result-card,
.result-card__actions {
  display: flex;
  gap: 0.75rem;
}

.dialog-header,
.section-header,
.result-card {
  justify-content: space-between;
  align-items: flex-start;
}

.dialog-header h3,
.dialog-header p,
.section-header h4,
.section-header p {
  margin: 0;
}

.dialog-header p,
.section-header p,
.field-help,
.result-score,
.info-message {
  color: var(--text-secondary);
}

.dialog-section {
  display: grid;
  gap: 1rem;
}

.search-row {
  align-items: end;
}

.search-field {
  flex: 1;
}

.search-button {
  flex-shrink: 0;
}

.field-block {
  display: grid;
  gap: 0.35rem;
  min-width: 0;
  color: var(--text-secondary);
  font-size: var(--text-sm);
}

.field-wide {
  grid-column: 1 / -1;
}

.form-grid {
  display: grid;
  gap: 0.75rem;
  grid-template-columns: repeat(auto-fit, minmax(14rem, 1fr));
}

.result-list {
  display: grid;
  gap: 0.75rem;
}

.result-card {
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  padding: 0.875rem;
  background: var(--bg-main);
}

.result-card__body {
  display: grid;
  gap: 0.35rem;
  min-width: 0;
}

.result-card__body p {
  margin: 0;
  white-space: pre-wrap;
  color: var(--text-secondary);
}

.result-card__actions {
  flex-wrap: wrap;
  justify-content: flex-end;
}

.memory-preview {
  display: grid;
  gap: 0.375rem;
}

.memory-preview span {
  color: var(--text-primary);
  font-size: var(--text-sm);
}

.memory-preview pre,
.success-state code {
  margin: 0;
  padding: 0.75rem;
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  background: var(--bg-main);
  white-space: pre-wrap;
  word-break: break-word;
  font: inherit;
}

.dialog-actions {
  justify-content: flex-end;
  flex-wrap: wrap;
}

.dialog-actions--start {
  justify-content: flex-start;
}

.form-error {
  margin: 0;
  color: var(--color-status-abandoned);
  font-size: var(--text-sm);
}

.mono {
  font-family: var(--font-mono);
  word-break: break-all;
}

.success-state {
  padding-top: 0.5rem;
}

@media (max-width: 720px) {
  .search-row,
  .dialog-header,
  .section-header,
  .result-card {
    flex-direction: column;
  }

  .result-card__actions,
  .dialog-actions {
    justify-content: stretch;
  }
}
</style>
