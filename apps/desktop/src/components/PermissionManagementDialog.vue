<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import {
  getDefaultPermissionLedger,
  type PermissionAuditEvent,
  type PermissionRejectionReason,
  type PersistedPermissionDecision,
} from '../lib/permission-ledger/ledger';

const props = defineProps<{ open: boolean }>();
const emit = defineEmits<{ close: [] }>();

const decisions = ref<PersistedPermissionDecision[]>([]);
const auditEvents = ref<PermissionAuditEvent[]>([]);
const confirmingReset = ref(false);

const groupedDecisions = computed(() => {
  const groups = new Map<string, PersistedPermissionDecision[]>();
  for (const decision of decisions.value) {
    const list = groups.get(decision.agent) ?? [];
    list.push(decision);
    groups.set(decision.agent, list);
  }
  return [...groups.entries()];
});

type AuditBadgeVariant = 'default' | 'destructive' | 'secondary' | 'outline';

function formatReason(reason: PermissionRejectionReason): string {
  switch (reason) {
    case 'token_invalid':
      return 'invalid token';
    case 'token_expired':
      return 'expired token';
    case 'option_mismatch':
      return 'wrong option';
    case 'scope_invalid':
      return 'invalid scope';
  }
}

function auditRowDescriptor(event: PermissionAuditEvent): {
  label: string;
  variant: AuditBadgeVariant;
} {
  if (event.source === 'main_auto_decide') {
    if (event.decision === 'allow') return { label: 'Auto-allowed', variant: 'default' };
    if (event.decision === 'reject') return { label: 'Auto-denied', variant: 'destructive' };
  }

  if (event.source === 'renderer_user_choice') {
    if (event.decision === 'allow') return { label: 'User allowed', variant: 'default' };
    if (event.decision === 'reject') return { label: 'User denied', variant: 'destructive' };
    if (event.decision === null && event.rejectionReason !== null) {
      return { label: `Rejected: ${formatReason(event.rejectionReason)}`, variant: 'destructive' };
    }
  }

  if (event.source === null) {
    if (event.decision === 'allow') return { label: 'Auto-allowed', variant: 'default' };
    if (event.decision === 'reject') return { label: 'Auto-denied', variant: 'destructive' };
  }

  return { label: 'Unknown', variant: 'outline' };
}

async function refresh(): Promise<void> {
  const ledger = getDefaultPermissionLedger();
  decisions.value = await ledger.listDecisions();
  auditEvents.value = await ledger.listAuditEvents();
}

async function revoke(id: number): Promise<void> {
  await getDefaultPermissionLedger().revokeDecision(id);
  await refresh();
}

async function resetAll(): Promise<void> {
  if (!confirmingReset.value) {
    confirmingReset.value = true;
    return;
  }
  await getDefaultPermissionLedger().resetDecisions();
  confirmingReset.value = false;
  await refresh();
}

watch(() => props.open, (open) => {
  if (open) void refresh();
});

onMounted(refresh);
</script>

<template>
  <Dialog :open="open" @update:open="value => { if (!value) emit('close') }">
    <DialogContent class="max-h-[80vh] max-w-3xl overflow-auto">
      <DialogHeader>
        <DialogTitle>Permission management</DialogTitle>
        <DialogDescription>
          Review remembered permission decisions and recent automatic decisions.
        </DialogDescription>
      </DialogHeader>

      <section>
        <div class="mb-2 flex items-center justify-between gap-3">
          <h3 class="m-0 text-base font-semibold">Remembered decisions</h3>
          <Button variant="destructive" size="sm" @click="resetAll">
            {{ confirmingReset ? 'Confirm reset' : 'Reset all permissions' }}
          </Button>
        </div>
        <div v-if="groupedDecisions.length === 0" class="rounded border border-border-subtle p-3 text-sm text-text-secondary">
          No remembered permissions.
        </div>
        <div v-for="[agent, rows] in groupedDecisions" :key="agent" class="mb-4 rounded border border-border-subtle">
          <div class="border-b border-border-subtle bg-bg-surface px-3 py-2 font-semibold">{{ agent }}</div>
          <div v-for="row in rows" :key="row.id" class="grid grid-cols-[1fr_auto] gap-3 border-b border-border-subtle px-3 py-2 last:border-b-0">
            <div class="min-w-0">
              <div class="flex flex-wrap items-center gap-2">
                <Badge :variant="row.decision === 'allow' ? 'default' : 'destructive'">{{ row.decision }}</Badge>
                <span class="font-semibold">{{ row.toolKind }}</span>
                <span class="text-sm text-text-secondary">{{ row.scope }}</span>
              </div>
              <div class="mt-1 break-all font-mono text-xs text-text-secondary">
                {{ row.canonicalTarget ?? 'any target' }}
              </div>
            </div>
            <Button variant="outline" size="sm" @click="revoke(row.id)">Revoke</Button>
          </div>
        </div>
      </section>

      <section class="mt-5">
        <h3 class="m-0 mb-2 text-base font-semibold">Audit log</h3>
        <div v-if="auditEvents.length === 0" class="rounded border border-border-subtle p-3 text-sm text-text-secondary">
          No automatic decisions yet.
        </div>
        <div v-for="event in auditEvents" :key="event.id" class="grid grid-cols-[auto_1fr] gap-3 border-b border-border-subtle py-2 text-sm last:border-b-0">
          <Badge :variant="auditRowDescriptor(event).variant">{{ auditRowDescriptor(event).label }}</Badge>
          <div class="min-w-0">
            <div>{{ event.agent }} · {{ event.toolKind }} · {{ new Date(event.createdAt).toLocaleString() }}</div>
            <div class="break-all font-mono text-xs text-text-secondary">
              {{ event.canonicalTarget ?? 'any target' }}
              <template v-if="event.source === 'renderer_user_choice' && event.tokenId">
                · token {{ event.tokenId.slice(0, 8) }}…
              </template>
            </div>
          </div>
        </div>
      </section>

      <DialogFooter>
        <Button variant="outline" @click="emit('close')">Close</Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
