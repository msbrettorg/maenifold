<script setup lang="ts">
import { Button } from '@/components/ui/button';
import { permissionGhostNotifications } from '../lib/permission-ledger/audit-events';
import { getDefaultPermissionLedger } from '../lib/permission-ledger/ledger';

async function revoke(permissionId: number | null): Promise<void> {
  if (permissionId == null) return;
  await getDefaultPermissionLedger().revokeDecision(permissionId);
}
</script>

<template>
  <div class="pointer-events-none fixed bottom-5 left-1/2 z-50 flex -translate-x-1/2 flex-col gap-2">
    <div
      v-for="notification in permissionGhostNotifications"
      :key="notification.id"
      class="pointer-events-auto flex items-center gap-3 rounded-lg border border-border-subtle bg-bg-surface px-3 py-2 text-sm text-text-primary shadow-lg"
    >
      <span>{{ notification.message }}</span>
      <Button
        v-if="notification.permissionId !== null"
        variant="ghost"
        size="xs"
        @click="revoke(notification.permissionId)"
      >
        Revoke
      </Button>
    </div>
  </div>
</template>
