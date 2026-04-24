<script setup lang="ts">
import { Download, Loader2, Trash2, Check, Info } from 'lucide-vue-next';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import {
  type ModelInfo,
  type PendingAction,
  formatModelSize,
  formatMtebScore,
  statusForModel,
} from '../composables/useEmbeddingsActions';

defineProps<{
  models: ModelInfo[];
  pending: Record<string, PendingAction | undefined>;
}>();

const emit = defineEmits<{
  download: [model: ModelInfo];
  activate: [model: ModelInfo];
  delete: [model: ModelInfo];
}>();
</script>

<template>
  <Table>
    <TableHeader>
      <TableRow>
        <TableHead>Model</TableHead>
        <TableHead class="hidden md:table-cell">Specs</TableHead>
        <TableHead class="hidden lg:table-cell">Quality</TableHead>
        <TableHead>Status</TableHead>
        <TableHead class="text-right">Actions</TableHead>
      </TableRow>
    </TableHeader>
    <TableBody>
      <TableRow v-for="model in models" :key="model.id">
        <TableCell class="align-top">
          <div class="flex flex-col gap-1 min-w-0">
            <span class="text-sm font-medium text-text-primary">{{ model.name }}</span>
            <span class="text-xs text-text-tertiary">{{ model.license }}</span>
          </div>
        </TableCell>
        <TableCell class="hidden md:table-cell align-top">
          <div class="flex flex-col gap-0.5 text-xs text-text-secondary tabular-nums">
            <span>{{ model.dimensions }}d · {{ model.maxTokens }} tok</span>
            <span class="text-text-tertiary">{{ formatModelSize(model.sizeBytes) }}</span>
          </div>
        </TableCell>
        <TableCell class="hidden lg:table-cell align-top">
          <div class="flex flex-col gap-1 text-xs text-text-secondary">
            <div
              v-if="model.mrlDimensions && model.mrlDimensions.length"
              class="flex items-center gap-1.5"
            >
              <span class="text-text-tertiary">MRL:</span>
              <span class="text-text-primary tabular-nums">{{ model.mrlDimensions.join(' / ') }}</span>
              <Tooltip>
                <TooltipTrigger as-child>
                  <Button
                    variant="ghost"
                    size="icon"
                    class="h-5 w-5 rounded-full text-text-tertiary hover:text-text-primary"
                    aria-label="What is MRL?"
                  >
                    <Info :size="12" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent side="top" class="max-w-[260px]">
                  Matryoshka Representation Learning — embeddings can be truncated to these smaller dimensions while keeping most retrieval quality.
                </TooltipContent>
              </Tooltip>
            </div>
            <div
              v-if="formatMtebScore(model.mtebRetrieval)"
              class="flex items-center gap-1.5"
            >
              <span class="text-text-tertiary">MTEB:</span>
              <span class="text-text-primary tabular-nums">{{ formatMtebScore(model.mtebRetrieval) }}</span>
            </div>
            <span
              v-if="!(model.mrlDimensions && model.mrlDimensions.length) && !formatMtebScore(model.mtebRetrieval)"
              class="text-text-tertiary"
            >—</span>
          </div>
        </TableCell>
        <TableCell class="align-top">
          <Badge :variant="statusForModel(model).variant">
            <Check v-if="model.isActive" :size="12" class="mr-1" />
            {{ statusForModel(model).label }}
          </Badge>
        </TableCell>
        <TableCell class="align-top">
          <div class="flex flex-wrap justify-end gap-2">
            <Button
              v-if="!model.installed"
              variant="default"
              size="sm"
              :disabled="pending[model.id] === 'downloading'"
              @click="emit('download', model)"
            >
              <Loader2
                v-if="pending[model.id] === 'downloading'"
                :size="14"
                class="animate-spin mr-1"
              />
              <Download v-else :size="14" class="mr-1" />
              {{
                pending[model.id] === 'downloading'
                  ? (model.downloadProgress && model.downloadProgress.total > 0
                      ? `Downloading… ${formatModelSize(model.downloadProgress.current)} / ${formatModelSize(model.downloadProgress.total)}`
                      : 'Downloading…')
                  : 'Download'
              }}
            </Button>
            <template v-else-if="!model.isActive">
              <Button
                variant="outline"
                size="sm"
                :disabled="!!pending[model.id]"
                @click="emit('activate', model)"
              >
                <Loader2
                  v-if="pending[model.id] === 'activating'"
                  :size="14"
                  class="animate-spin mr-1"
                />
                {{ pending[model.id] === 'activating' ? 'Activating…' : 'Activate' }}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                class="text-danger hover:text-danger"
                :disabled="!!pending[model.id]"
                @click="emit('delete', model)"
              >
                <Loader2
                  v-if="pending[model.id] === 'deleting'"
                  :size="14"
                  class="animate-spin mr-1"
                />
                <Trash2 v-else :size="14" class="mr-1" />
                {{ pending[model.id] === 'deleting' ? 'Deleting…' : 'Delete' }}
              </Button>
            </template>
            <span v-else class="text-xs text-text-tertiary self-center">
              Active — switch first to manage
            </span>
          </div>
        </TableCell>
      </TableRow>
    </TableBody>
  </Table>
</template>
