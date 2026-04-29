<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { Button } from '@/components/ui/button';
import PermissionDialog from './PermissionDialog.vue';
import type { PermissionRequest } from '../lib/types';
import type { PermissionScope } from '../lib/permission-ledger/ledger';
import { canonicalPermissionTarget, normalizePermissionToolKind } from '../lib/permission-ledger/equivalence-key';

const props = defineProps<{
  request: PermissionRequest;
  totalPending: number;
}>();

const emit = defineEmits<{
  select: [optionId: string, scope?: PermissionScope];
  cancel: [];
}>();

const pendingAlwaysOptionId = ref<string | null>(null);
const selectedScope = ref<PermissionScope>('agent');

const pendingOption = computed(() =>
  props.request.options.find((option) => option.optionId === pendingAlwaysOptionId.value) ?? null
);

function defaultScope(request: PermissionRequest): PermissionScope {
  const kind = normalizePermissionToolKind(request.toolCall.kind);
  const target = canonicalPermissionTarget(request, kind) ?? '';
  if (kind === 'execute' || kind === 'delete') return 'session';
  if (kind === 'read' && (target.startsWith('memory://') || target.startsWith('~/maenifold/'))) {
    return 'forever';
  }
  if (kind === 'read' || kind === 'fetch') return 'agent';
  return 'agent';
}

watch(
  () => props.request.toolCall.toolCallId,
  () => {
    pendingAlwaysOptionId.value = null;
    selectedScope.value = defaultScope(props.request);
  },
  { immediate: true }
);

function handleSelect(optionId: string): void {
  const option = props.request.options.find((candidate) => candidate.optionId === optionId);
  if (option?.kind.endsWith('_always')) {
    pendingAlwaysOptionId.value = optionId;
    selectedScope.value = defaultScope(props.request);
    return;
  }
  emit('select', optionId);
}

function confirmScope(): void {
  if (!pendingAlwaysOptionId.value) return;
  emit('select', pendingAlwaysOptionId.value, selectedScope.value);
}
</script>

<template>
  <PermissionDialog
    :request="request"
    :total-pending="totalPending"
    @select="handleSelect"
    @cancel="emit('cancel')"
  />

  <div
    v-if="pendingOption"
    data-test="permission-scope-picker"
    class="fixed inset-0 z-[60] flex items-center justify-center bg-black/30"
  >
    <div class="w-[90%] max-w-md rounded-lg border border-border-subtle bg-bg-surface p-4 shadow-xl">
      <div class="mb-3">
        <h3 class="m-0 text-lg font-semibold text-text-primary">Remember this decision?</h3>
        <p class="m-0 mt-1 text-sm text-text-secondary">
          Choose how long Maenifold should apply "{{ pendingOption.name }}".
        </p>
      </div>

      <div class="flex flex-col gap-2">
        <Button
          variant="outline"
          class="h-auto justify-start whitespace-normal p-3 text-left"
          data-scope="session"
          :aria-pressed="selectedScope === 'session'"
          @click="selectedScope = 'session'"
        >
          <span>
            <span class="block font-semibold">This session</span>
            <span class="block text-sm text-text-secondary">Forget the decision when this session ends.</span>
          </span>
        </Button>
        <Button
          variant="outline"
          class="h-auto justify-start whitespace-normal p-3 text-left"
          data-scope="agent"
          :aria-pressed="selectedScope === 'agent'"
          @click="selectedScope = 'agent'"
        >
          <span>
            <span class="block font-semibold">This agent</span>
            <span class="block text-sm text-text-secondary">Apply across sessions for the same agent.</span>
          </span>
        </Button>
        <Button
          variant="outline"
          class="h-auto justify-start whitespace-normal p-3 text-left"
          data-scope="forever"
          :aria-pressed="selectedScope === 'forever'"
          @click="selectedScope = 'forever'"
        >
          <span>
            <span class="block font-semibold">Forever</span>
            <span class="block text-sm text-text-secondary">Keep until you revoke it from permission management.</span>
          </span>
        </Button>
      </div>

      <div class="mt-4 flex justify-end gap-2">
        <Button variant="outline" @click="pendingAlwaysOptionId = null">Back</Button>
        <Button data-test="confirm-permission-scope" @click="confirmScope">Confirm</Button>
      </div>
    </div>
  </div>
</template>
