<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { computed, onMounted } from 'vue';
import type {
  PipelineNode,
  PromptNode,
  SequentialThinkingNode,
  WorkflowNode,
} from '../lib/pipeline-types';
import { getNodeType } from '../lib/pipeline-types';
import { usePipelineStore } from '../stores/pipeline';

const props = defineProps<{
  node: PipelineNode;
}>();

const emit = defineEmits<{
  (e: 'update:node', next: PipelineNode): void;
}>();

const store = usePipelineStore();

const NONE_OPTION = '__none__';

const adoptableNode = computed<WorkflowNode | PromptNode | SequentialThinkingNode | null>(
  () => getAdoptableNode(props.node),
);
const roleAssetOptions = computed(() => store.roleAssets);
const colorAssetOptions = computed(() => store.colorAssets);
const perspectiveAssetOptions = computed(() => store.perspectiveAssets);

onMounted(async () => {
  if (
    roleAssetOptions.value.length === 0
    && colorAssetOptions.value.length === 0
    && perspectiveAssetOptions.value.length === 0
  ) {
    await store.refreshAdoptAssets();
  }
});

function getAdoptableNode(
  node: PipelineNode,
): WorkflowNode | PromptNode | SequentialThinkingNode | null {
  const nodeType = getNodeType(node);
  return nodeType === 'tool'
    ? null
    : (node as WorkflowNode | PromptNode | SequentialThinkingNode);
}

function normalizeOptionalSelectValue(value: unknown): string | undefined {
  const nextValue = String(value ?? '').trim();
  if (nextValue.length === 0 || nextValue === NONE_OPTION) {
    return undefined;
  }
  return nextValue;
}

function updateAdoptField(field: 'role' | 'color' | 'perspective', value: unknown): void {
  const selectedValue = normalizeOptionalSelectValue(value);
  const node = getAdoptableNode(props.node);
  if (!node) return;

  const nextAdopt = { ...(node.adopt ?? {}) };
  if (selectedValue) {
    nextAdopt[field] = selectedValue;
  } else {
    delete nextAdopt[field];
  }

  const { adopt: _adopt, ...rest } = node;
  const nextNode = Object.keys(nextAdopt).length > 0
    ? { ...rest, adopt: nextAdopt }
    : rest;
  emit('update:node', nextNode as PipelineNode);
}

function updateRequiresRole(value: unknown): void {
  const selectedValue = normalizeOptionalSelectValue(value);
  const node = getAdoptableNode(props.node);
  if (!node) return;
  const { requiresRole: _requiresRole, ...rest } = node;
  const nextNode = selectedValue
    ? { ...rest, requiresRole: selectedValue }
    : rest;
  emit('update:node', nextNode as PipelineNode);
}
</script>

<template>
  <Card class="border-border bg-surface-container">
    <CardHeader class="space-y-1.5">
      <CardTitle class="font-display text-sm text-text-primary">Adopt configuration</CardTitle>
      <CardDescription class="text-xs text-text-secondary">
        Apply role, color, and perspective before node dispatch.
      </CardDescription>
    </CardHeader>
    <CardContent class="grid gap-3">
      <label class="pipeline-inspector__field">
        <span>Role</span>
        <Select
          :model-value="adoptableNode?.adopt?.role ?? NONE_OPTION"
          @update:model-value="updateAdoptField('role', $event)"
        >
          <SelectTrigger>
            <SelectValue placeholder="Select role" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem :value="NONE_OPTION">None</SelectItem>
            <SelectItem
              v-for="option in roleAssetOptions"
              :key="`role-${option.id}`"
              :value="option.id"
            >
              {{ option.id }}
            </SelectItem>
          </SelectContent>
        </Select>
      </label>
      <label class="pipeline-inspector__field">
        <span>Color</span>
        <Select
          :model-value="adoptableNode?.adopt?.color ?? NONE_OPTION"
          @update:model-value="updateAdoptField('color', $event)"
        >
          <SelectTrigger>
            <SelectValue placeholder="Select color" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem :value="NONE_OPTION">None</SelectItem>
            <SelectItem
              v-for="option in colorAssetOptions"
              :key="`color-${option.id}`"
              :value="option.id"
            >
              {{ option.id }}
            </SelectItem>
          </SelectContent>
        </Select>
      </label>
      <label class="pipeline-inspector__field">
        <span>Perspective</span>
        <Select
          :model-value="adoptableNode?.adopt?.perspective ?? NONE_OPTION"
          @update:model-value="updateAdoptField('perspective', $event)"
        >
          <SelectTrigger>
            <SelectValue placeholder="Select perspective" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem :value="NONE_OPTION">None</SelectItem>
            <SelectItem
              v-for="option in perspectiveAssetOptions"
              :key="`perspective-${option.id}`"
              :value="option.id"
            >
              {{ option.id }}
            </SelectItem>
          </SelectContent>
        </Select>
      </label>
      <label class="pipeline-inspector__field">
        <span>Requires role</span>
        <Select
          :model-value="adoptableNode?.requiresRole ?? NONE_OPTION"
          @update:model-value="updateRequiresRole($event)"
        >
          <SelectTrigger>
            <SelectValue placeholder="Select required role" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem :value="NONE_OPTION">None</SelectItem>
            <SelectItem
              v-for="option in roleAssetOptions"
              :key="`requires-role-${option.id}`"
              :value="option.id"
            >
              {{ option.id }}
            </SelectItem>
          </SelectContent>
        </Select>
      </label>
    </CardContent>
  </Card>
</template>
