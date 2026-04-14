import { onBeforeUnmount, onMounted } from 'vue';

type UseComposerKeyboardOptions = {
  closeContextMenu: () => void;
};

export function useComposerKeyboard(options: UseComposerKeyboardOptions): void {
  function handleWindowPointerDown(event: PointerEvent): void {
    const target = event.target;
    if (!(target instanceof Element)) {
      options.closeContextMenu();
      return;
    }

    if (!target.closest('.composer-context-menu')) {
      options.closeContextMenu();
    }
  }

  function handleWindowKeyDown(event: KeyboardEvent): void {
    if (event.key === 'Escape') {
      options.closeContextMenu();
    }
  }

  onMounted(() => {
    window.addEventListener('pointerdown', handleWindowPointerDown);
    window.addEventListener('keydown', handleWindowKeyDown);
  });

  onBeforeUnmount(() => {
    window.removeEventListener('pointerdown', handleWindowPointerDown);
    window.removeEventListener('keydown', handleWindowKeyDown);
  });
}
