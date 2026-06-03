<script setup lang="ts">
// Promote-to-memory dialog. Teleported into `#pipeline-overlay` and driven by
// the `usePromoteToMemory` singleton (the SemanticZoom overlay calls
// `openFor(node)`). Three steps:
//   search  — pick an existing note to update, or "create new",
//   editor  — edit title/content/section; the WikiLink pre-gate blocks a
//             write/edit with no [[concept]],
//   confirm — success summary.
// An update reads the target WITH a checksum and sends it; a stale checksum
// re-reads the note, preserves the in-progress edit, and shows a review banner.
import { BookPlus, FilePlus2, Search, X } from '@lucide/vue';
import { usePromoteToMemory } from '../../../composables/usePromoteToMemory';
import type { PromoteMode } from '../../../lib/promote/memoryOps';

const promote = usePromoteToMemory();
const {
  open,
  step,
  mode,
  sourceLabel,
  draft,
  searchQuery,
  searchResults,
  searching,
  submitting,
  errorMessage,
  staleBanner,
  gateMessage,
  confirmation,
  requiresWikiLink,
  isUpdate,
} = promote;

// Update operations (an existing note was chosen). 'create' is handled separately.
const UPDATE_MODES: { value: Exclude<PromoteMode, 'create'>; label: string }[] = [
  { value: 'append', label: 'Append' },
  { value: 'prepend', label: 'Prepend' },
  { value: 'replace_section', label: 'Replace section' },
  { value: 'find_replace', label: 'Find & replace' },
];

function onModeChange(event: Event): void {
  promote.setMode((event.target as HTMLSelectElement).value as PromoteMode);
}
</script>

<template>
  <Teleport defer to="#pipeline-overlay">
    <div
      v-if="open"
      class="promote-scrim"
      role="dialog"
      aria-modal="true"
      aria-label="Promote to memory"
      @click.self="promote.close()"
    >
      <div class="promote-panel">
        <header class="promote-header">
          <div class="promote-title">
            <BookPlus :size="16" :stroke-width="2" />
            <strong>Promote to memory</strong>
            <span class="promote-source">{{ sourceLabel }}</span>
          </div>
          <button type="button" class="promote-close" aria-label="Close" @click="promote.close()">
            <X :size="16" :stroke-width="2" />
          </button>
        </header>

        <!-- Step rail -->
        <nav class="promote-steps" aria-label="Promote steps">
          <span class="promote-step" :class="{ active: step === 'search' }">1 · Choose target</span>
          <span class="promote-step" :class="{ active: step === 'editor' }">2 · Edit</span>
          <span class="promote-step" :class="{ active: step === 'confirm' }">3 · Confirm</span>
        </nav>

        <div class="promote-body">
          <!-- STEP 1: search / create-new -->
          <section v-if="step === 'search'" class="promote-search">
            <button type="button" class="promote-createnew" @click="promote.chooseCreateNew()">
              <FilePlus2 :size="15" :stroke-width="2" />
              <span>Create a new memory note</span>
            </button>

            <p class="promote-or">or update an existing note</p>

            <form class="promote-searchbar" @submit.prevent="promote.runSearch()">
              <Search :size="14" :stroke-width="2" />
              <input
                v-model="searchQuery"
                type="search"
                class="promote-searchinput"
                placeholder="Search memories…"
                aria-label="Search memories"
              />
              <button type="submit" class="promote-searchbtn" :disabled="searching">
                {{ searching ? 'Searching…' : 'Search' }}
              </button>
            </form>

            <p v-if="errorMessage" class="promote-error" role="status">{{ errorMessage }}</p>

            <ul v-if="searchResults.length > 0" class="promote-results">
              <li v-for="result in searchResults" :key="result.path ?? result.title">
                <button
                  type="button"
                  class="promote-result"
                  :disabled="submitting || !result.path"
                  @click="promote.chooseExisting(result)"
                >
                  <span class="promote-result-title">{{ result.title ?? result.path }}</span>
                  <span v-if="result.snippet" class="promote-result-snippet">{{ result.snippet }}</span>
                </button>
              </li>
            </ul>
            <p v-else-if="!searching && searchQuery.trim()" class="promote-hint">No matches.</p>
          </section>

          <!-- STEP 2: editor -->
          <section v-else-if="step === 'editor'" class="promote-editor">
            <p v-if="staleBanner" class="promote-stale" role="status">{{ staleBanner }}</p>

            <div v-if="isUpdate" class="promote-field">
              <label class="promote-label" for="promote-mode">Update mode</label>
              <select id="promote-mode" class="promote-select" :value="mode" @change="onModeChange">
                <option v-for="option in UPDATE_MODES" :key="option.value" :value="option.value">
                  {{ option.label }}
                </option>
              </select>
            </div>

            <div v-if="!isUpdate" class="promote-field">
              <label class="promote-label" for="promote-title">Title</label>
              <input
                id="promote-title"
                v-model="draft.title"
                type="text"
                class="promote-input"
                placeholder="Memory note title"
              />
            </div>

            <div v-if="!isUpdate" class="promote-field">
              <label class="promote-label" for="promote-folder">Folder (optional)</label>
              <input
                id="promote-folder"
                v-model="draft.folder"
                type="text"
                class="promote-input"
                placeholder="e.g. research"
              />
            </div>

            <div v-if="mode === 'replace_section'" class="promote-field">
              <label class="promote-label" for="promote-section">Section name</label>
              <input
                id="promote-section"
                v-model="draft.sectionName"
                type="text"
                class="promote-input"
                placeholder="Exact heading text to replace"
              />
            </div>

            <div v-if="mode === 'find_replace'" class="promote-field">
              <label class="promote-label" for="promote-find">Find text</label>
              <input
                id="promote-find"
                v-model="draft.findText"
                type="text"
                class="promote-input"
                placeholder="Exact text to find (must match once)"
              />
            </div>

            <div class="promote-field">
              <label class="promote-label" for="promote-content">
                Content
                <span v-if="requiresWikiLink" class="promote-req">· requires a [[concept]] link</span>
              </label>
              <textarea
                id="promote-content"
                v-model="draft.content"
                class="promote-textarea"
                rows="10"
              ></textarea>
            </div>

            <p v-if="gateMessage" class="promote-gate" role="status">{{ gateMessage }}</p>
            <p v-if="errorMessage" class="promote-error" role="status">{{ errorMessage }}</p>

            <div class="promote-actions">
              <button type="button" class="promote-secondary" @click="promote.goToStep('search')">
                Back
              </button>
              <button type="button" class="promote-primary" :disabled="submitting" @click="promote.submit()">
                {{ submitting ? 'Saving…' : isUpdate ? 'Update memory' : 'Create memory' }}
              </button>
            </div>
          </section>

          <!-- STEP 3: confirm -->
          <section v-else class="promote-confirm">
            <p class="promote-success">{{ confirmation }}</p>
            <div class="promote-actions">
              <button type="button" class="promote-primary" @click="promote.close()">Done</button>
            </div>
          </section>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.promote-scrim {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 1.5rem;
  background: color-mix(in srgb, var(--bg-main) 60%, transparent);
  backdrop-filter: blur(2px);
  z-index: 9;
}

.promote-panel {
  display: flex;
  flex-direction: column;
  width: min(640px, 100%);
  max-height: 100%;
  border: 1px solid var(--border-color);
  border-radius: 12px;
  background: color-mix(in srgb, var(--bg-main) 96%, var(--bg-sidebar));
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.28);
  overflow: hidden;
}

.promote-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  padding: 0.7rem 0.85rem;
  border-bottom: 1px solid var(--border-color);
  background: color-mix(in srgb, var(--bg-sidebar) 55%, transparent);
  color: var(--text-accent);
}

.promote-title {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  min-width: 0;
}

.promote-title strong {
  font-size: 0.9rem;
  color: var(--text-primary);
}

.promote-source {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 0.74rem;
  color: var(--text-muted);
}

.promote-close {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0.3rem;
  border: 1px solid transparent;
  border-radius: 7px;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
}

.promote-close:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.promote-steps {
  display: flex;
  gap: 0.5rem;
  padding: 0.55rem 0.85rem;
  border-bottom: 1px solid var(--border-color);
}

.promote-step {
  font-size: 0.72rem;
  font-weight: 600;
  color: var(--text-muted);
}

.promote-step.active {
  color: var(--text-accent);
}

.promote-body {
  flex: 1;
  min-height: 0;
  padding: 0.9rem;
  overflow-y: auto;
}

.promote-search,
.promote-editor,
.promote-confirm {
  display: flex;
  flex-direction: column;
  gap: 0.65rem;
}

.promote-createnew {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.55rem 0.7rem;
  border: 1px solid color-mix(in srgb, var(--text-accent) 35%, transparent);
  border-radius: 8px;
  background: color-mix(in srgb, var(--text-accent) 10%, transparent);
  color: var(--text-accent);
  cursor: pointer;
  font: inherit;
  font-size: 0.82rem;
  font-weight: 600;
}

.promote-createnew:hover {
  background: color-mix(in srgb, var(--text-accent) 18%, transparent);
}

.promote-or {
  margin: 0;
  font-size: 0.74rem;
  color: var(--text-muted);
  text-align: center;
}

.promote-searchbar {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.35rem 0.5rem;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: var(--bg-main);
  color: var(--text-muted);
}

.promote-searchinput {
  flex: 1;
  min-width: 0;
  border: none;
  background: transparent;
  color: var(--text-primary);
  font: inherit;
  font-size: 0.82rem;
  outline: none;
}

.promote-searchbtn {
  flex-shrink: 0;
  padding: 0.3rem 0.6rem;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  background: var(--bg-hover);
  color: var(--text-primary);
  cursor: pointer;
  font: inherit;
  font-size: 0.76rem;
}

.promote-searchbtn:disabled {
  opacity: 0.55;
  cursor: default;
}

.promote-results {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  margin: 0;
  padding: 0;
  list-style: none;
}

.promote-result {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
  width: 100%;
  padding: 0.5rem 0.6rem;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: color-mix(in srgb, var(--bg-main) 96%, var(--bg-sidebar));
  color: var(--text-primary);
  cursor: pointer;
  text-align: left;
}

.promote-result:hover:not(:disabled) {
  background: var(--bg-hover);
}

.promote-result:disabled {
  opacity: 0.55;
  cursor: default;
}

.promote-result-title {
  font-size: 0.83rem;
  font-weight: 600;
}

.promote-result-snippet {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 0.74rem;
  color: var(--text-secondary);
}

.promote-hint {
  margin: 0;
  font-size: 0.78rem;
  color: var(--text-muted);
}

.promote-field {
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
}

.promote-label {
  font-size: 0.76rem;
  font-weight: 600;
  color: var(--text-secondary);
}

.promote-req {
  font-weight: 500;
  color: var(--text-muted);
}

.promote-input,
.promote-select,
.promote-textarea {
  width: 100%;
  padding: 0.5rem 0.6rem;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: var(--bg-main);
  color: var(--text-primary);
  font: inherit;
  font-size: 0.82rem;
  outline: none;
}

.promote-textarea {
  resize: vertical;
  line-height: 1.45;
  font-family: var(--font-mono, monospace);
}

.promote-input:focus,
.promote-select:focus,
.promote-textarea:focus {
  border-color: color-mix(in srgb, var(--text-accent) 45%, var(--border-color));
}

.promote-stale {
  margin: 0;
  padding: 0.5rem 0.6rem;
  border-radius: 7px;
  background: color-mix(in srgb, var(--node-status-active, #e0af68) 18%, var(--bg-main));
  color: var(--text-primary);
  font-size: 0.78rem;
}

.promote-gate,
.promote-error {
  margin: 0;
  padding: 0.45rem 0.55rem;
  border-radius: 6px;
  background: color-mix(in srgb, var(--bg-danger) 12%, var(--bg-main));
  color: var(--text-primary);
  font-size: 0.76rem;
}

.promote-success {
  margin: 0;
  font-size: 0.86rem;
  color: var(--text-primary);
}

.promote-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.5rem;
}

.promote-secondary {
  padding: 0.45rem 0.8rem;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  font: inherit;
  font-size: 0.8rem;
}

.promote-secondary:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.promote-primary {
  padding: 0.45rem 0.85rem;
  border: 1px solid color-mix(in srgb, var(--text-accent) 40%, transparent);
  border-radius: 8px;
  background: color-mix(in srgb, var(--text-accent) 14%, transparent);
  color: var(--text-accent);
  cursor: pointer;
  font: inherit;
  font-size: 0.8rem;
  font-weight: 600;
}

.promote-primary:hover:not(:disabled) {
  background: color-mix(in srgb, var(--text-accent) 22%, transparent);
}

.promote-primary:disabled {
  opacity: 0.55;
  cursor: default;
}
</style>
