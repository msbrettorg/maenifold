<script setup lang="ts">
import PermissionDialog from './PermissionDialog.vue';
import type { PermissionScope } from '../lib/permission-ledger/ledger';
import type { PermissionRequest } from '../lib/types';

const props = defineProps<{
  request: PermissionRequest;
  totalPending: number;
}>();

const emit = defineEmits<{
  select: [optionId: string, scope?: PermissionScope];
  cancel: [];
}>();

function handleSelect(optionId: string, scope?: PermissionScope): void {
  if (scope === undefined) {
    emit('select', optionId);
    return;
  }
  emit('select', optionId, scope);
}
</script>

<template>
  <PermissionDialog
    :request="request"
    :total-pending="totalPending"
    @select="handleSelect"
    @cancel="emit('cancel')"
  />
</template>
