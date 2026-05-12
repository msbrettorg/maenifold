import { onScopeDispose, ref, watch, type Ref } from 'vue';

const PREFIX = 'maenifold.chatDraft.';
const DEBOUNCE_MS = 400;
const MAX_BYTES = 50_000;

function draftKey(sessionId: string): string {
  return `${PREFIX}${sessionId}`;
}

function readLocal(sessionId: string): string | null {
  try {
    return localStorage.getItem(draftKey(sessionId));
  } catch {
    return null;
  }
}

function writeLocal(sessionId: string, text: string): void {
  try {
    const value = text.length > MAX_BYTES ? text.slice(0, MAX_BYTES) : text;
    localStorage.setItem(draftKey(sessionId), value);
  } catch {
    /* silent degrade on localStorage write failure */
  }
}

function removeLocal(sessionId: string): void {
  try {
    localStorage.removeItem(draftKey(sessionId));
  } catch {
    /* silent degrade on localStorage write failure */
  }
}

export function useChatDraft(sessionId: Ref<string | null>): {
  inputText: Ref<string>;
  clearDraft(): void;
} {
  const inputText = ref('');
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  function flushPending(id: string): void {
    if (debounceTimer !== null) {
      clearTimeout(debounceTimer);
      debounceTimer = null;
      const text = inputText.value;
      if (text) { writeLocal(id, text); } else { removeLocal(id); }
    }
  }

  function clearDraft(): void {
    const id = sessionId.value;
    if (id) removeLocal(id);
  }

  watch(
    sessionId,
    (newId, oldId) => {
      if (oldId !== undefined && oldId !== null) flushPending(oldId);
      else if (debounceTimer !== null) { clearTimeout(debounceTimer); debounceTimer = null; }
      inputText.value = (newId ? readLocal(newId) : null) ?? '';
    },
    { immediate: true },
  );

  watch(inputText, (text) => {
    const id = sessionId.value;
    if (!id) return;
    if (debounceTimer !== null) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      if (text) { writeLocal(id, text); } else { removeLocal(id); }
      debounceTimer = null;
    }, DEBOUNCE_MS);
  });

  onScopeDispose(() => {
    if (debounceTimer !== null) { clearTimeout(debounceTimer); debounceTimer = null; }
  });

  return { inputText, clearDraft };
}
