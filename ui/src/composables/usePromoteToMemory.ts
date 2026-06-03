// Promote-to-memory controller — the decoupling seam between the SemanticZoom
// overlay (which launches the flow) and the PromoteToMemoryDialog (which is
// teleported into `#pipeline-overlay`, a sibling subtree). Both call
// `usePromoteToMemory()` and share this module-level singleton state.
//
// Three-step state machine: search → editor → confirm.
//   - search: find an existing note to update, or choose "create new".
//   - editor: edit title/content/section; the WikiLink pre-gate blocks a
//     write/edit that has no `[[concept]]`.
//   - confirm: success summary.
//
// Update safety:
//   - An update reads the target note WITH its checksum; the checksum is sent
//     on the EditMemory so a concurrent change is detected.
//   - On a stale checksum the note is re-read, the in-progress editor content is
//     PRESERVED, and a "memory changed — review & reapply" banner is shown so the
//     user re-confirms against the fresh baseline instead of clobbering it.

import { computed, reactive, ref } from 'vue';
import type { ViewNode } from '../lib/pipeline-types';
import {
  isPromptViewNode,
  isSequentialThinkingViewNode,
  isToolViewNode,
} from '../lib/pipeline-types';
import { wikiLinkGate } from '../lib/promote/wikiLinkGate';
import {
  buildEditRequest,
  buildWriteRequest,
  editMemory,
  readMemoryWithChecksum,
  searchExistingMemories,
  writeMemory,
  type PromoteMode,
} from '../lib/promote/memoryOps';
import type { MemorySearchResult } from '../stores/knowledge';

export type PromoteStep = 'search' | 'editor' | 'confirm';

interface PromoteTarget {
  identifier: string;
  title: string;
  checksum: string;
  baseline: string;
}

// Which modes require a WikiLink in their content. Mirrors the backend, where
// Write + append/prepend/replace_section enforce a `[[concept]]` but find_replace
// does not. Empty content is always rejected separately.
const WIKILINK_REQUIRED_MODES: ReadonlySet<PromoteMode> = new Set([
  'create',
  'append',
  'prepend',
  'replace_section',
]);

// --- module-level singleton state ------------------------------------------

const open = ref(false);
const step = ref<PromoteStep>('search');
const mode = ref<PromoteMode>('create');

const sourceLabel = ref('');
const draft = reactive({
  title: '',
  content: '',
  sectionName: '',
  findText: '',
  folder: '',
});

const searchQuery = ref('');
const searchResults = ref<MemorySearchResult[]>([]);
const searching = ref(false);

const target = ref<PromoteTarget | null>(null);
const submitting = ref(false);
const errorMessage = ref<string | null>(null);
const staleBanner = ref<string | null>(null);
const gateMessage = ref<string | null>(null);
const confirmation = ref<string | null>(null);

const STALE_BANNER =
  'This memory changed since you opened it. The latest version is loaded below — review and reapply your edit.';

/** Pull a sensible seed body out of whatever node surface launched the flow. */
function seedFromNode(node: ViewNode): string {
  if (isPromptViewNode(node) || isSequentialThinkingViewNode(node)) {
    if (typeof node.output === 'string' && node.output.trim()) return node.output;
    if (typeof node.assembledPrompt === 'string') return node.assembledPrompt ?? '';
  }
  if (isToolViewNode(node)) {
    if (typeof node.output === 'string' && node.output.trim()) return node.output;
  }
  return typeof node.output === 'string' ? node.output : '';
}

function resetState(): void {
  step.value = 'search';
  mode.value = 'create';
  draft.title = '';
  draft.content = '';
  draft.sectionName = '';
  draft.findText = '';
  draft.folder = '';
  searchQuery.value = '';
  searchResults.value = [];
  searching.value = false;
  target.value = null;
  submitting.value = false;
  errorMessage.value = null;
  staleBanner.value = null;
  gateMessage.value = null;
  confirmation.value = null;
}

export function usePromoteToMemory() {
  const requiresWikiLink = computed(() => WIKILINK_REQUIRED_MODES.has(mode.value));
  const isUpdate = computed(() => mode.value !== 'create');

  function openFor(node: ViewNode): void {
    resetState();
    const label = node.displayLabel ?? node.nodeId;
    sourceLabel.value = label;
    draft.title = label;
    draft.content = seedFromNode(node);
    open.value = true;
  }

  function close(): void {
    open.value = false;
    resetState();
  }

  function goToStep(next: PromoteStep): void {
    step.value = next;
  }

  async function runSearch(): Promise<void> {
    const query = searchQuery.value.trim();
    if (!query) {
      searchResults.value = [];
      return;
    }
    searching.value = true;
    errorMessage.value = null;
    try {
      searchResults.value = await searchExistingMemories(query);
    } catch (error) {
      errorMessage.value = error instanceof Error ? error.message : String(error);
      searchResults.value = [];
    } finally {
      searching.value = false;
    }
  }

  /** Choose "create a new note" and advance to the editor. */
  function chooseCreateNew(): void {
    mode.value = 'create';
    target.value = null;
    staleBanner.value = null;
    step.value = 'editor';
  }

  /** Choose an existing note to update; reads it WITH checksum for safe edits. */
  async function chooseExisting(
    result: MemorySearchResult,
    nextMode: Exclude<PromoteMode, 'create'> = 'append'
  ): Promise<void> {
    const identifier = result.path;
    if (!identifier) {
      errorMessage.value = 'That memory has no identifier and cannot be edited.';
      return;
    }
    submitting.value = true;
    errorMessage.value = null;
    try {
      const file = await readMemoryWithChecksum(identifier);
      target.value = {
        identifier,
        title: file.title ?? result.title ?? identifier,
        checksum: file.checksum ?? '',
        baseline: file.content ?? '',
      };
      mode.value = nextMode;
      staleBanner.value = null;
      step.value = 'editor';
    } catch (error) {
      errorMessage.value = error instanceof Error ? error.message : String(error);
    } finally {
      submitting.value = false;
    }
  }

  function setMode(next: PromoteMode): void {
    mode.value = next;
    gateMessage.value = null;
  }

  /** Run the WikiLink pre-gate for the current mode + content. */
  function checkGate(): boolean {
    gateMessage.value = null;
    if (!draft.content.trim()) {
      gateMessage.value = 'Content cannot be empty.';
      return false;
    }
    if (requiresWikiLink.value) {
      const result = wikiLinkGate(draft.content);
      if (!result.ok) {
        gateMessage.value = result.message;
        return false;
      }
    }
    return true;
  }

  async function submit(): Promise<void> {
    if (submitting.value) return;
    if (!checkGate()) return;

    if (mode.value === 'create') {
      const title = draft.title.trim();
      if (!title) {
        errorMessage.value = 'A title is required to create a new memory.';
        return;
      }
    } else if (!target.value) {
      errorMessage.value = 'Choose an existing memory to update first.';
      return;
    } else if (mode.value === 'replace_section' && !draft.sectionName.trim()) {
      errorMessage.value = 'A section name is required to replace a section.';
      return;
    } else if (mode.value === 'find_replace' && !draft.findText.trim()) {
      errorMessage.value = 'Provide the exact text to find and replace.';
      return;
    }

    submitting.value = true;
    errorMessage.value = null;
    try {
      const outcome =
        mode.value === 'create'
          ? await writeMemory(buildWriteRequest(draft.title, draft.content, draft.folder))
          : await editMemory(
              buildEditRequest(mode.value, target.value!.identifier, draft.content, target.value!.checksum, {
                findText: draft.findText,
                sectionName: draft.sectionName,
              })
            );

      if (outcome.kind === 'ok') {
        confirmation.value =
          mode.value === 'create'
            ? `Created memory "${draft.title.trim()}".`
            : `Updated "${target.value?.title ?? 'memory'}".`;
        staleBanner.value = null;
        step.value = 'confirm';
        return;
      }

      if (outcome.kind === 'stale') {
        await recoverFromStale();
        return;
      }

      errorMessage.value = outcome.message ?? 'The memory operation failed.';
    } catch (error) {
      errorMessage.value = error instanceof Error ? error.message : String(error);
    } finally {
      submitting.value = false;
    }
  }

  // Re-read the note to get the fresh checksum + baseline WITHOUT discarding the
  // user's in-progress edit, then surface the "review & reapply" banner.
  async function recoverFromStale(): Promise<void> {
    const current = target.value;
    if (!current) {
      errorMessage.value = 'The memory changed and could not be reloaded.';
      return;
    }
    try {
      const file = await readMemoryWithChecksum(current.identifier);
      target.value = {
        ...current,
        checksum: file.checksum ?? '',
        baseline: file.content ?? '',
        title: file.title ?? current.title,
      };
      staleBanner.value = STALE_BANNER;
      step.value = 'editor';
    } catch (error) {
      errorMessage.value = error instanceof Error ? error.message : String(error);
    }
  }

  return {
    // state
    open,
    step,
    mode,
    sourceLabel,
    draft,
    searchQuery,
    searchResults,
    searching,
    target,
    submitting,
    errorMessage,
    staleBanner,
    gateMessage,
    confirmation,
    // derived
    requiresWikiLink,
    isUpdate,
    // actions
    openFor,
    close,
    goToStep,
    runSearch,
    chooseCreateNew,
    chooseExisting,
    setMode,
    checkGate,
    submit,
  };
}
