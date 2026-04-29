import type { Ref } from 'vue';
import type { useMemoryApi } from '../composables/useMemoryApi';

type MemoryApi = ReturnType<typeof useMemoryApi>;

interface MemoryActionDeps {
  activeTabUri: Ref<string | null>;
  loadRootFolders(): Promise<void>;
  loadTab(uri: string, force?: boolean): Promise<void>;
  setActiveTab(uri: string): void;
  closeTab(uri: string): void;
  openNote(uri: string): void;
}

export function createKnowledgeMemoryActions(deps: MemoryActionDeps, memoryApi: MemoryApi) {
  async function createNote(input: { title: string; folder?: string; content: string }): Promise<string> {
    const uri = await memoryApi.writeMemory(input);
    await deps.loadRootFolders();
    deps.setActiveTab(uri);
    return uri;
  }

  async function reloadNote(uri: string): Promise<void> {
    await deps.loadTab(uri, true);
  }

  async function saveNoteContent(input: {
    uri: string;
    previousContent: string;
    content: string;
    checksum: string | null;
  }): Promise<void> {
    if (!input.checksum) {
      throw new Error('A checksum is required before updating this note.');
    }
    await memoryApi.editMemory({
      identifier: input.uri,
      findText: input.previousContent,
      content: input.content,
      checksum: input.checksum,
    });
    await reloadNote(input.uri);
  }

  async function forceSaveNoteContent(uri: string, content: string): Promise<void> {
    const latest = await memoryApi.readMemory(uri);
    if (!latest.checksum) {
      throw new Error('A checksum is required before updating this note.');
    }
    await memoryApi.editMemory({
      identifier: uri,
      findText: latest.content,
      content,
      checksum: latest.checksum,
    });
    await reloadNote(uri);
  }

  async function deleteNote(uri: string): Promise<void> {
    await memoryApi.deleteMemory(uri);
    await deps.loadRootFolders();
    if (deps.activeTabUri.value === uri) {
      deps.closeTab(uri);
    }
  }

  async function moveNote(source: string, destination: string): Promise<string> {
    const uri = await memoryApi.moveMemory({ source, destination });
    await deps.loadRootFolders();
    if (deps.activeTabUri.value === source) {
      deps.closeTab(source);
      deps.openNote(uri);
    }
    return uri;
  }

  return {
    createNote,
    reloadNote,
    saveNoteContent,
    forceSaveNoteContent,
    deleteNote,
    moveNote,
  };
}
