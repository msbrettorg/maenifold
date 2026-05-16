import { onScopeDispose } from 'vue';
import { useAppShellStore } from '../stores/app-shell';

export function useGlobalKeymap(): void {
  const store = useAppShellStore();

  function isEditableTarget(): boolean {
    const activeElement = document.activeElement;
    return activeElement instanceof HTMLInputElement || activeElement instanceof HTMLTextAreaElement;
  }

  function handleKeyDown(event: KeyboardEvent): void {
    if (event.shiftKey && event.key === '?') {
      if (isEditableTarget()) return;
      store.cheatsheetOpen = true;
      return;
    }

    if (!(event.metaKey || event.ctrlKey) || event.key.toLowerCase() !== 'k') return;
    event.preventDefault();
    store.commandPaletteOpen = !store.commandPaletteOpen;
  }

  window.addEventListener('keydown', handleKeyDown, true);
  onScopeDispose(() => window.removeEventListener('keydown', handleKeyDown, true));
}
