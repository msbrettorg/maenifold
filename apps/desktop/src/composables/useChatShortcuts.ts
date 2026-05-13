import type { ComputedRef } from 'vue';

interface ChatShortcutHandlers {
  submit: () => void;
  recallLast: () => void;
  cancelTurn: () => void;
  requestClear: () => void;
  isStreaming: ComputedRef<boolean>;
  isInputEmpty: ComputedRef<boolean>;
}

export function useChatShortcuts(handlers: ChatShortcutHandlers) {
  function consume(event: KeyboardEvent, action: () => void): boolean {
    event.preventDefault();
    action();
    return true;
  }

  function handleShortcut(event: KeyboardEvent): boolean {
    const usesCommandModifier = event.metaKey || event.ctrlKey;
    if (event.key === 'Enter' && usesCommandModifier && !handlers.isInputEmpty.value) {
      return consume(event, handlers.submit);
    }
    if (event.key === 'ArrowUp' && handlers.isInputEmpty.value) {
      return consume(event, handlers.recallLast);
    }
    if (event.key === 'Escape' && handlers.isStreaming.value) {
      return consume(event, handlers.cancelTurn);
    }
    if (event.key.toLowerCase() === 'l' && usesCommandModifier) {
      return consume(event, handlers.requestClear);
    }
    return false;
  }

  return { handleShortcut };
}
