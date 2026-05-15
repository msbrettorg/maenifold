<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { Loader2, RefreshCw } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { TooltipProvider } from '@/components/ui/tooltip';
import {
  type ModelInfo,
  useEmbeddingsActions,
} from '../composables/useEmbeddingsActions';
import SettingsEmbeddingsTable from './SettingsEmbeddingsTable.vue';
import SettingsEmbeddingsConfirmDialog from './SettingsEmbeddingsConfirmDialog.vue';

const {
  loading,
  error,
  status,
  models,
  pending,
  regenerating,
  activeModel,
  hasDimensionWarning,
  initialLoad,
  downloadModel,
  activateModel,
  deleteModel,
  regenerateEmbeddings,
} = useEmbeddingsActions();

type DialogType = 'activate' | 'delete' | 'regenerate';
const dialog = ref<{ type: DialogType; modelId: string } | null>(null);

const dialogModel = computed<ModelInfo | null>(() => {
  if (!dialog.value) return null;
  return models.value.find((m) => m.id === dialog.value!.modelId) ?? null;
});

const dialogOpen = computed({
  get: () => dialog.value !== null,
  set: (open: boolean) => {
    if (!open) dialog.value = null;
  },
});

function openActivate(model: ModelInfo): void {
  dialog.value = { type: 'activate', modelId: model.id };
}

function openDelete(model: ModelInfo): void {
  dialog.value = { type: 'delete', modelId: model.id };
}

function openRegenerate(): void {
  dialog.value = { type: 'regenerate', modelId: '' };
}

async function confirmDialog(payload: { type: 'activate' | 'delete' | 'regenerate'; modelId: string }): Promise<void> {
  dialog.value = null;
  if (payload.type === 'regenerate') {
    await regenerateEmbeddings();
    return;
  }
  const model = models.value.find((m) => m.id === payload.modelId);
  if (!model) return;
  if (payload.type === 'activate') {
    await activateModel(model);
  } else {
    await deleteModel(model);
  }
}

onMounted(() => {
  void initialLoad();
});
</script>

<template>
  <TooltipProvider :delay-duration="200">
    <section class="flex flex-col gap-4" role="region" aria-label="Embeddings">
      <Card>
        <CardHeader>
          <div class="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
            <div class="flex flex-col gap-1">
              <CardTitle class="text-base">Embeddings</CardTitle>
              <CardDescription>
                Embedding models for semantic search and concept similarity.
              </CardDescription>
            </div>
            <Button
              variant="outline"
              size="sm"
              class="shrink-0"
              :disabled="regenerating"
              @click="openRegenerate"
            >
              <Loader2 v-if="regenerating" :size="14" class="animate-spin mr-1" />
              <RefreshCw v-else :size="14" class="mr-1" />
              {{ regenerating ? 'Regenerating…' : 'Regenerate embeddings' }}
            </Button>
          </div>
        </CardHeader>

        <CardContent class="flex flex-col gap-4">
          <p
            v-if="error"
            class="m-0 text-sm text-danger"
            role="alert"
          >{{ error }}</p>

          <div
            v-if="loading"
            class="text-sm text-text-secondary"
          >Loading model information…</div>

          <template v-else>
            <Card>
              <CardHeader>
                <CardTitle class="text-sm">Current model</CardTitle>
                <CardDescription>
                  <template v-if="activeModel">
                    {{ activeModel.name }} — {{ activeModel.dimensions }}d,
                    {{ activeModel.maxTokens }} max tokens
                  </template>
                  <template v-else-if="status?.loaded">
                    {{ status.model?.filename }} — {{ status.model?.dimensions }}d,
                    {{ status.model?.maxTokens }} max tokens
                  </template>
                  <template v-else>No model loaded</template>
                </CardDescription>
              </CardHeader>
            </Card>

            <SettingsEmbeddingsTable
              :models="models"
              :pending="pending"
              @download="downloadModel"
              @activate="openActivate"
              @delete="openDelete"
            />

            <div
              v-if="hasDimensionWarning"
              class="rounded-md border border-warning/40 bg-warning/10 px-4 py-3 text-sm text-warning"
              role="status"
            >
              Switching to a model with different dimensions requires regenerating all embeddings.
            </div>
          </template>
        </CardContent>
      </Card>

      <SettingsEmbeddingsConfirmDialog
        :open="dialogOpen"
        :type="dialog?.type ?? null"
        :model-id="dialog?.modelId ?? ''"
        :model-name="dialogModel?.name ?? null"
        @update:open="(v: boolean) => (dialogOpen = v)"
        @confirm="confirmDialog"
      />
    </section>
  </TooltipProvider>
</template>
