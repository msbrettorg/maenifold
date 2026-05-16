import { ref } from 'vue';

type UseFileDropOptions = {
  onFiles: (files: File[]) => void;
  enabled: () => boolean;
};

function hasFiles(types: readonly string[] | DOMStringList): boolean {
  return Array.from(types).includes('Files');
}

export function useFileDrop({ onFiles, enabled }: UseFileDropOptions) {
  const dropZoneActive = ref(false);

  function handleDragOver(event: DragEvent): void {
    if (!enabled() || !event.dataTransfer || !hasFiles(event.dataTransfer.types)) {
      return;
    }
    event.preventDefault();
    event.dataTransfer.dropEffect = 'copy';
    dropZoneActive.value = true;
  }

  function handleDragLeave(): void {
    dropZoneActive.value = false;
  }

  function handleDrop(event: DragEvent): void {
    dropZoneActive.value = false;
    if (!enabled()) {
      return;
    }
    const files = Array.from(event.dataTransfer?.files ?? []);
    if (!files.length) return;
    event.preventDefault();
    onFiles(files);
  }

  function handlePaste(event: ClipboardEvent): void {
    if (!enabled()) {
      return;
    }
    const files = Array.from(event.clipboardData?.files ?? []);
    if (!files.length) {
      return;
    }
    event.preventDefault();
    onFiles(files);
  }

  return {
    dropZoneActive,
    handleDragOver,
    handleDragLeave,
    handleDrop,
    handlePaste,
  };
}
