<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { storeToRefs } from 'pinia';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { Textarea } from '@/components/ui/textarea';
import { useMcpClient } from '@/composables/useMcpClient';
import {
  formatTeamsConnectionState,
  parseTeamsStatusPayload,
  type TeamsConnectionConfig,
  type TeamsIdentity,
  type TeamsMonitorConfig,
  type TeamsMonitorStatus,
} from '@/composables/useTeamsStatus';
import { useConfigStore } from '@/stores/config';

const NO_AGENT_VALUE = '__none__';
const configStore = useConfigStore();
const { agentNames } = storeToRefs(configStore);
const { callTool, connect, connected } = useMcpClient();

const isLoading = ref(true);
const isSaving = ref(false);
const isBusy = ref(false);
const error = ref('');
const statusMessage = ref('');
const runtime = ref(parseTeamsStatusPayload({}));
const identity = ref<TeamsIdentity | null>(null);
const connection = ref<TeamsConnectionConfig>({});
const enabled = ref(false);
const keyword = ref('@maenifold');
const listenScope = ref('wildcard');
const monitoredConversations = ref('');
const replyBehavior = ref('sourceConversation');
const replyPrefix = ref('maenifold: ');
const dispatchTimeoutMinutes = ref('10');
const dispatchAgent = ref('');

const dispatchAgentValue = computed(() => dispatchAgent.value || NO_AGENT_VALUE);
const connectionStatusLabel = computed(() => formatTeamsConnectionState(runtime.value.connectionState));
const authStatusLabel = computed(() => formatTeamsConnectionState(runtime.value.authState));
const identityTitle = computed(() => identity.value?.displayName || identity.value?.upn || 'Not detected');
const identitySubtitle = computed(() => identity.value?.mri || 'Use Detect identity to read the signed-in Teams token.');
const runtimeError = computed(() => error.value || runtime.value.lastError || runtime.value.lastAuthError || '');
const regionLabel = computed(() => connection.value.chatsvcRegion || identity.value?.chatsvcRegion || 'amer');

function stringArray(value: unknown): string[] {
  return Array.isArray(value) ? value.filter((entry): entry is string => typeof entry === 'string') : [];
}
function parseLines(value: string): string[] {
  return Array.from(new Set(value.split('\n').map((entry) => entry.trim()).filter(Boolean)));
}
function statusMessageFrom(data: unknown): string {
  return data && typeof data === 'object' && typeof (data as { message?: unknown }).message === 'string'
    ? String((data as { message: string }).message)
    : '';
}
function applyStatus(data: unknown): void {
  if (!data || typeof data !== 'object') return;
  const obj = data as { config?: TeamsMonitorConfig };
  const nextRuntime = parseTeamsStatusPayload(data);
  const config = obj.config ?? {};
  runtime.value = nextRuntime;
  enabled.value = nextRuntime.enabled;
  identity.value = config.identity ?? null;
  connection.value = config.connection ?? {};
  keyword.value = config.keyword || '@maenifold';
  listenScope.value = config.listenScope || 'wildcard';
  monitoredConversations.value = stringArray(config.monitoredConversations).join('\n');
  replyBehavior.value = config.replyBehavior || 'sourceConversation';
  replyPrefix.value = config.replyPrefix ?? 'maenifold: ';
  dispatchTimeoutMinutes.value = String(config.dispatchTimeoutMinutes ?? 10);
  dispatchAgent.value = typeof config.dispatchAgent === 'string' ? config.dispatchAgent : '';
}
async function ensureConnected() {
  if (!connected.value) await connect();
}
async function invokeTeams(action: string, payload: Record<string, unknown> = {}) {
  await ensureConnected();
  const status = await callTool<TeamsMonitorStatus>('TeamsMonitor', { action, ...payload });
  applyStatus(status);
  statusMessage.value = statusMessageFrom(status);
}
async function loadStatus() {
  isLoading.value = true;
  error.value = '';
  try {
    if (!configStore.loading && agentNames.value.length === 0) await configStore.loadConfig();
    await invokeTeams('status');
  } catch (loadError) {
    error.value = loadError instanceof Error ? loadError.message : String(loadError);
  } finally {
    isLoading.value = false;
  }
}
function parseTimeoutMinutes(): number {
  const value = Number(dispatchTimeoutMinutes.value);
  if (!Number.isInteger(value) || value <= 0) throw new Error('Dispatch timeout must be a positive integer.');
  return value;
}
async function runAction(action: string) {
  isBusy.value = true;
  error.value = '';
  statusMessage.value = '';
  try {
    await invokeTeams(action);
  } catch (actionError) {
    error.value = actionError instanceof Error ? actionError.message : String(actionError);
  } finally {
    isBusy.value = false;
  }
}
async function handleToggle(nextValue: boolean) {
  const previous = enabled.value;
  enabled.value = nextValue;
  isBusy.value = true;
  error.value = '';
  statusMessage.value = '';
  try {
    await invokeTeams(nextValue ? 'enable' : 'disable');
  } catch (toggleError) {
    enabled.value = previous;
    error.value = toggleError instanceof Error ? toggleError.message : String(toggleError);
  } finally {
    isBusy.value = false;
  }
}
async function handleSave() {
  isSaving.value = true;
  error.value = '';
  statusMessage.value = '';
  try {
    await invokeTeams('configure', {
      keyword: keyword.value.trim(),
      listenScope: listenScope.value,
      monitoredConversations: parseLines(monitoredConversations.value),
      replyBehavior: replyBehavior.value,
      replyPrefix: replyPrefix.value,
      dispatchTimeoutMinutes: parseTimeoutMinutes(),
      dispatchAgent: dispatchAgent.value || null,
    });
    statusMessage.value ||= 'Teams monitor settings saved.';
  } catch (saveError) {
    error.value = saveError instanceof Error ? saveError.message : String(saveError);
  } finally {
    isSaving.value = false;
  }
}
function updateDispatchAgent(value: string | number | bigint | Record<string, unknown> | null) {
  const next = String(value ?? NO_AGENT_VALUE);
  dispatchAgent.value = next === NO_AGENT_VALUE ? '' : next;
}
function updateListenScope(value: string | number | bigint | Record<string, unknown> | null) {
  listenScope.value = String(value ?? 'wildcard');
}
function updateReplyBehavior(value: string | number | bigint | Record<string, unknown> | null) {
  replyBehavior.value = String(value ?? 'sourceConversation');
}

onMounted(() => { void loadStatus(); });
</script>

<template>
  <section class="flex flex-col gap-4" role="form" aria-label="Teams monitor settings">
    <div class="flex flex-col gap-1">
      <h3 class="text-base font-semibold text-text-primary">Teams monitor</h3>
      <p class="text-sm text-text-secondary">
        Reuses the signed-in Teams desktop session token to monitor and dispatch.
      </p>
    </div>
    <Alert v-if="runtimeError" variant="destructive">
      <AlertTitle>Teams monitor needs attention</AlertTitle>
      <AlertDescription>{{ runtimeError }}</AlertDescription>
    </Alert>
    <p v-else-if="statusMessage" class="text-sm text-primary" role="status">{{ statusMessage }}</p>
    <div v-if="isLoading" class="rounded-md border border-border bg-muted/40 p-4 text-sm text-muted-foreground">
      Loading Teams monitor settings…
    </div>
    <template v-else>
      <div class="grid gap-4 lg:grid-cols-[1fr_auto]">
        <div class="rounded-md border border-border bg-muted/40 p-4">
          <div class="flex items-start justify-between gap-4">
            <div class="min-w-0">
              <p class="text-sm font-semibold text-text-primary">{{ identityTitle }}</p>
              <p class="mt-1 break-all text-xs text-muted-foreground">{{ identitySubtitle }}</p>
            </div>
            <Badge variant="outline">{{ authStatusLabel }}</Badge>
          </div>
          <div class="mt-3 grid gap-2 text-xs text-muted-foreground sm:grid-cols-3">
            <span>Region: {{ regionLabel }}</span>
            <span>UPN: {{ identity?.upn || 'unknown' }}</span>
            <span>Token expires: {{ runtime.expiresAt || identity?.expiresAt || 'unknown' }}</span>
          </div>
        </div>
        <div class="flex flex-wrap items-center gap-2 lg:justify-end">
          <Button type="button" variant="secondary" :disabled="isBusy || isSaving" @click="runAction('detect_identity')">
            Detect identity
          </Button>
          <Button type="button" variant="secondary" :disabled="isBusy || isSaving" @click="runAction('refresh')">
            Refresh
          </Button>
          <Button type="button" variant="outline" :disabled="isBusy || isSaving" @click="runAction('sign_out')">
            Sign out
          </Button>
        </div>
      </div>

      <div class="flex items-center justify-between gap-4 rounded-md border border-border bg-muted/40 p-4 max-sm:flex-col max-sm:items-stretch">
        <div class="flex flex-col gap-1">
          <Label for="teams-monitor-enabled" class="text-sm font-semibold">Enable monitor</Label>
          <span class="text-sm text-muted-foreground">Starts the real Trouter listener with the detected token and identity.</span>
        </div>
        <Switch
          id="teams-monitor-enabled"
          :model-value="enabled"
          :disabled="isBusy || isSaving"
          @update:model-value="handleToggle"
        />
      </div>

      <div class="grid gap-4 lg:grid-cols-2">
        <div class="flex flex-col gap-2 rounded-md border border-border bg-muted/40 p-4">
          <Label for="teams-monitor-keyword" class="text-sm font-semibold">Trigger keyword</Label>
          <Input id="teams-monitor-keyword" v-model="keyword" placeholder="@maenifold" :disabled="isSaving" />
        </div>
        <div class="flex flex-col gap-2 rounded-md border border-border bg-muted/40 p-4">
          <Label for="teams-monitor-agent" class="text-sm font-semibold">Dispatch agent</Label>
          <Select :model-value="dispatchAgentValue" :disabled="isSaving" @update:model-value="updateDispatchAgent">
            <SelectTrigger id="teams-monitor-agent"><SelectValue placeholder="Select an agent" /></SelectTrigger>
            <SelectContent>
              <SelectItem :value="NO_AGENT_VALUE">No agent</SelectItem>
              <SelectItem v-for="agent in agentNames" :key="agent" :value="agent">{{ agent }}</SelectItem>
            </SelectContent>
          </Select>
        </div>
        <div class="flex flex-col gap-2 rounded-md border border-border bg-muted/40 p-4">
          <Label for="teams-monitor-scope" class="text-sm font-semibold">Listen scope</Label>
          <Select :model-value="listenScope" :disabled="isSaving" @update:model-value="updateListenScope">
            <SelectTrigger id="teams-monitor-scope"><SelectValue /></SelectTrigger>
            <SelectContent>
              <SelectItem value="wildcard">All conversations</SelectItem>
              <SelectItem value="self">Self chat only</SelectItem>
              <SelectItem value="custom">Custom conversation IDs</SelectItem>
            </SelectContent>
          </Select>
        </div>
        <div class="flex flex-col gap-2 rounded-md border border-border bg-muted/40 p-4">
          <Label for="teams-monitor-reply" class="text-sm font-semibold">Reply target</Label>
          <Select :model-value="replyBehavior" :disabled="isSaving" @update:model-value="updateReplyBehavior">
            <SelectTrigger id="teams-monitor-reply"><SelectValue /></SelectTrigger>
            <SelectContent>
              <SelectItem value="sourceConversation">Source conversation</SelectItem>
              <SelectItem value="selfChat">Self chat</SelectItem>
            </SelectContent>
          </Select>
        </div>
        <div class="flex flex-col gap-2 rounded-md border border-border bg-muted/40 p-4">
          <Label for="teams-monitor-prefix" class="text-sm font-semibold">Reply prefix</Label>
          <Input id="teams-monitor-prefix" v-model="replyPrefix" placeholder="maenifold: " :disabled="isSaving" />
        </div>
        <div class="flex flex-col gap-2 rounded-md border border-border bg-muted/40 p-4">
          <Label for="teams-monitor-timeout" class="text-sm font-semibold">Dispatch timeout minutes</Label>
          <Input id="teams-monitor-timeout" v-model="dispatchTimeoutMinutes" type="number" min="1" step="1" :disabled="isSaving" />
        </div>
      </div>

      <div v-if="listenScope === 'custom'" class="flex flex-col gap-2 rounded-md border border-border bg-muted/40 p-4">
        <Label for="teams-monitor-conversations" class="text-sm font-semibold">Monitored conversation IDs</Label>
        <Textarea id="teams-monitor-conversations" v-model="monitoredConversations" placeholder="19:thread&#10;48:notes" :disabled="isSaving" />
        <p class="text-sm text-muted-foreground">One Teams conversation ID per line. Use self-chat for 48:notes.</p>
      </div>

      <div class="flex items-center justify-between gap-4 rounded-md border border-border bg-muted/40 p-4 max-sm:flex-col max-sm:items-stretch">
        <div class="flex flex-col gap-1">
          <span class="text-sm font-semibold text-text-primary">Connection status</span>
          <span class="text-sm text-muted-foreground">{{ connectionStatusLabel }} · {{ runtime.matchCount }} matches · {{ runtime.activeDispatches }} active dispatches</span>
        </div>
        <Button type="button" :disabled="isSaving || isBusy" @click="handleSave">
          {{ isSaving ? 'Saving…' : 'Save Teams monitor' }}
        </Button>
      </div>
    </template>
  </section>
</template>
