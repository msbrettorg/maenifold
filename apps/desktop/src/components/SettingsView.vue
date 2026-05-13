<script setup lang="ts">
// Settings tab: direct editor over ~/maenifold/maenifold.json via Electron IPC.
// Daemon hot-reloads via api/config_watch.rs — no MCP round-trip on save.
//
// Reference: https://github.com/cloydlau/json-editor-vue#props
//   v-model binds the value directly. With mode="text" + default stringified:true
//   the input/output stays as the raw JSON string — what you see is what you get.
import { onBeforeUnmount, onMounted, ref, watch } from 'vue';
import JsonEditorVue from 'json-editor-vue';
import { Mode } from 'vanilla-jsoneditor';
import 'vanilla-jsoneditor/themes/jse-theme-dark.css';
import { Button } from '@/components/ui/button';
import { useConfigStore } from '../stores/config';
import { useUnsavedChangesGuard } from '../composables/useUnsavedChangesGuard';
import SettingsEmbeddings from './SettingsEmbeddings.vue';
import SettingsMcpServers from './SettingsMcpServers.vue';

const configStore = useConfigStore();
const guard = useUnsavedChangesGuard({ title: 'Discard settings changes?' });

const editorText = ref<string>('');
const filePath = ref('');
const exists = ref(false);
const isLoading = ref(true);
const isSaving = ref(false);
const isDirty = ref(false);
const feedback = ref<{ type: 'success' | 'error' | 'info'; message: string } | null>(null);

let unsubscribeWatcher: (() => void) | null = null;
let suppressNextDirty = false;

async function load() {
  isLoading.value = true;
  feedback.value = null;
  try {
    const api = window.electronAPI?.configRead;
    if (!api) throw new Error('Electron config IPC unavailable');
    const result = await api();
    filePath.value = result.path;
    exists.value = result.exists;
    suppressNextDirty = true;
    editorText.value = result.content;
    isDirty.value = false;
    guard.markClean();
  } catch (error) {
    feedback.value = {
      type: 'error',
      message: error instanceof Error ? error.message : String(error),
    };
  } finally {
    isLoading.value = false;
  }
}

async function save() {
  isSaving.value = true;
  feedback.value = null;
  try {
    JSON.parse(editorText.value);
    await configStore.writeRaw(editorText.value);
    isDirty.value = false;
    guard.markClean();
    feedback.value = { type: 'success', message: `Saved to ${filePath.value}.` };
  } catch (error) {
    feedback.value = {
      type: 'error',
      message: error instanceof Error ? error.message : String(error),
    };
  } finally {
    isSaving.value = false;
  }
}

async function revert() {
  if (!(await guard.confirmIfDirty())) return;
  await load();
  feedback.value = { type: 'info', message: 'Reverted to file on disk.' };
}

watch(editorText, () => {
  if (suppressNextDirty) {
    suppressNextDirty = false;
    return;
  }
  isDirty.value = true;
  guard.markDirty();
});

let detachBeforeUnload: (() => void) | null = null;
onMounted(async () => {
  detachBeforeUnload = guard.attachBeforeUnload();
  await load();
  if (window.electronAPI?.onConfigChanged) {
    unsubscribeWatcher = window.electronAPI.onConfigChanged(() => {
      if (isDirty.value) {
        feedback.value = {
          type: 'info',
          message: 'File changed on disk — your unsaved edits are preserved. Revert to discard them.',
        };
        return;
      }
      void load();
    });
  }
});

onBeforeUnmount(() => {
  detachBeforeUnload?.();
  unsubscribeWatcher?.();
});
</script>

<template>
  <div class="flex h-full min-h-0 flex-col gap-6 overflow-y-auto p-6">
    <section class="flex flex-col gap-3">
      <header class="flex flex-col gap-1">
        <h2 class="text-lg font-semibold text-text-primary">Configuration</h2>
        <p class="text-sm text-text-secondary">
          Edit <code class="font-mono text-xs">{{ filePath || 'maenifold.json' }}</code> directly.
          The daemon hot-reloads on save.
        </p>
      </header>

      <div
        v-if="feedback"
        class="rounded border px-4 py-2 text-sm"
        :class="{
          'border-success/40 bg-success/10 text-success': feedback.type === 'success',
          'border-destructive/40 bg-destructive/10 text-destructive': feedback.type === 'error',
          'border-brand/40 bg-brand-light text-text-primary': feedback.type === 'info',
        }"
        role="status"
      >
        {{ feedback.message }}
      </div>

      <div v-if="isLoading" class="rounded border border-border-subtle bg-bg-elevated p-4 text-sm text-text-secondary">
        Loading…
      </div>

      <div v-else class="jse-theme-dark h-[520px] overflow-hidden rounded border border-border-subtle">
        <JsonEditorVue
          v-model="editorText"
          :mode="Mode.text"
          :main-menu-bar="true"
          :navigation-bar="true"
          :status-bar="true"
          :indentation="2"
          class="h-full"
        />
      </div>

      <footer class="flex flex-wrap items-center gap-2">
        <Button type="button" :disabled="!isDirty || isSaving || isLoading" @click="save">
          {{ isSaving ? 'Saving…' : 'Save' }}
        </Button>
        <Button type="button" variant="outline" :disabled="!isDirty || isSaving || isLoading" @click="revert">
          Revert
        </Button>
        <span v-if="!exists" class="text-xs text-text-tertiary">
          File does not exist yet — saving will create it.
        </span>
        <span v-else-if="isDirty" class="text-xs text-warning">Unsaved changes</span>
        <span v-else class="text-xs text-text-tertiary">In sync with disk</span>
      </footer>
    </section>

    <section class="flex flex-col gap-3">
      <SettingsMcpServers />
    </section>

    <section class="flex flex-col gap-3">
      <header class="flex flex-col gap-1">
        <h2 class="text-lg font-semibold text-text-primary">Embeddings</h2>
        <p class="text-sm text-text-secondary">
          Manage local embedding models used for semantic search. The active model is recorded under
          <code class="font-mono text-xs">settings.vector.active_model</code>.
        </p>
      </header>
      <SettingsEmbeddings />
    </section>
  </div>
</template>
