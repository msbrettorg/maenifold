import { onMounted, onUnmounted } from 'vue';
import type { ComputedRef, Ref } from 'vue';

export interface GateShortcut {
  key: string;
  description: string;
}

export const SHORTCUTS: GateShortcut[] = [
  { key: 'j / ↓', description: 'Next task' },
  { key: 'k / ↑', description: 'Previous task' },
  { key: 'Enter', description: 'Open selected task' },
  { key: 'Escape', description: 'Close detail panel' },
  { key: '/', description: 'Focus search' },
  { key: 'n', description: 'New task' },
  { key: 'p', description: 'Pass current stage' },
  { key: 'f', description: 'Fail current stage' },
  { key: 'a', description: 'Append note' },
  { key: 'r', description: 'Refresh snapshot' },
  { key: '?', description: 'Toggle shortcut help' },
];

interface UseGateKeyboardOptions {
  isActive: ComputedRef<boolean>;
  selectedTaskKey: Ref<string>;
  searchInputEl: Ref<HTMLElement | null>;
  onNext: () => void;
  onPrev: () => void;
  onOpen: () => void;
  onClose: () => void;
  onNew: () => void;
  onPass: () => void;
  onFail: () => void;
  onAppend: () => void;
  onRefresh: () => void;
  onHelp: () => void;
}

function isEditableTarget(): boolean {
  const el = document.activeElement;
  if (!el) return false;
  if ((el as HTMLElement).isContentEditable) return true;
  const tag = el.tagName;
  return tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT';
}

export function useGateKeyboard(options: UseGateKeyboardOptions): void {
  const {
    isActive, selectedTaskKey, searchInputEl,
    onNext, onPrev, onOpen, onClose, onNew,
    onPass, onFail, onAppend, onRefresh, onHelp,
  } = options;

  function handleKeydown(e: KeyboardEvent): void {
    if (!isActive.value) return;
    if (e.ctrlKey || e.metaKey || e.altKey) return;
    if (isEditableTarget() && e.key !== 'Escape') return;

    const hasTask = Boolean(selectedTaskKey.value);

    switch (e.key) {
      case 'j':
      case 'ArrowDown':
        e.preventDefault();
        onNext();
        break;
      case 'k':
      case 'ArrowUp':
        e.preventDefault();
        onPrev();
        break;
      case 'Enter':
        if (hasTask) { e.preventDefault(); onOpen(); }
        break;
      case 'Escape':
        e.preventDefault();
        onClose();
        break;
      case '/':
        e.preventDefault();
        searchInputEl.value?.focus();
        break;
      case 'n':
        e.preventDefault();
        onNew();
        break;
      case 'p':
        if (hasTask) { e.preventDefault(); onPass(); }
        break;
      case 'f':
        if (hasTask) { e.preventDefault(); onFail(); }
        break;
      case 'a':
        if (hasTask) { e.preventDefault(); onAppend(); }
        break;
      case 'r':
        e.preventDefault();
        onRefresh();
        break;
      case '?':
        e.preventDefault();
        onHelp();
        break;
    }
  }

  onMounted(() => window.addEventListener('keydown', handleKeydown));
  onUnmounted(() => window.removeEventListener('keydown', handleKeydown));
}
