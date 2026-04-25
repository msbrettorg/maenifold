<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { storeToRefs } from 'pinia';
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
import { useConfigStore } from '@/stores/config';

interface TeamsMonitorStatus {
  enabled: boolean;
  connectionState: string;
  config?: {
    keywords?: string[];
    senderAllowList?: string[] | null;
    senderDenyList?: string[] | null;
    cooldownSeconds?: number;
    dispatchAgent?: string | null;
  };
  message?: string;
}

const NO_AGENT_VALUE = '__none__';

const configStore = useConfigStore();
const { agentNames } = storeToRefs(configStore);
const { callTool, connect, connected } = useMcpClient();

const isLoading = ref(true);
const isSaving = ref(false);
const isToggling = ref(false);
const error = ref('');
const statusMessage = ref('');

const enabled = ref(false);
const connectionState = ref('disabled');
const keywordsInput = ref('');
const dispatchAgent = ref('');
const cooldownSeconds = ref('300');
const senderAllowList = ref('');
const senderDenyList = ref('');

const keywordTags = computed(() => parseCommaSeparated(keywordsInput.value));
const dispatchAgentValue = computed(() => dispatchAgent.value || NO_AGENT_VALUE);
const connectionStatusLabel = computed(() => formatConnectionState(connectionState.value));

function parseCommaSeparated(value: string): string[] {
  return Array.from(new Set(
    value
      .split(',')
      .map((entry) => entry.trim())
      .filter(Boolean),
  ));
}

function parseLineSeparated(value: string): string[] {
  return Array.from(new Set(
    value
      .split('\n')
      .map((entry) => entry.trim())
      .filter(Boolean),
  ));
}

function formatConnectionState(value: string): string {
  return value
    .split('_')
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ');
}

function applyStatus(status: TeamsMonitorStatus) {
  enabled.value = status.enabled;
  connectionState.value = status.connectionState;
  keywordsInput.value = (status.config?.keywords ?? []).join(', ');
  dispatchAgent.value = status.config?.dispatchAgent ?? '';
  cooldownSeconds.value = String(status.config?.cooldownSeconds ?? 300);
  senderAllowList.value = (status.config?.senderAllowList ?? []).join('\n');
  senderDenyList.value = (status.config?.senderDenyList ?? []).join('\n');
}

async function ensureConnected() {
  if (!connected.value) {
    await connect();
  }
}

async function loadStatus() {
  isLoading.value = true;
  error.value = '';

  try {
    if (!configStore.loading && agentNames.value.length === 0) {
      await configStore.loadConfig();
    }
    await ensureConnected();
    const status = await callTool<TeamsMonitorStatus>('TeamsMonitor', { action: 'status' });
    applyStatus(status);
    statusMessage.value = status.message ?? '';
  } catch (loadError) {
    error.value = loadError instanceof Error ? loadError.message : String(loadError);
  } finally {
    isLoading.value = false;
  }
}

function parseCooldown(): number {
  const value = Number(cooldownSeconds.value);
  if (!Number.isInteger(value) || value < 0) {
    throw new Error('Cooldown must be a non-negative integer.');
  }
  return value;
}

async function handleToggle(nextValue: boolean) {
  if (isToggling.value) {
    return;
  }

  const previousValue = enabled.value;
  enabled.value = nextValue;
  isToggling.value = true;
  error.value = '';
  statusMessage.value = '';

  try {
    await ensureConnected();
    const status = await callTool<TeamsMonitorStatus>('TeamsMonitor', {
      action: nextValue ? 'enable' : 'disable',
    });
    applyStatus(status);
    statusMessage.value = status.message ?? '';
  } catch (toggleError) {
    enabled.value = previousValue;
    error.value = toggleError instanceof Error ? toggleError.message : String(toggleError);
  } finally {
    isToggling.value = false;
  }
}

async function handleSave() {
  isSaving.value = true;
  error.value = '';
  statusMessage.value = '';

  try {
    await ensureConnected();
    const status = await callTool<TeamsMonitorStatus>('TeamsMonitor', {
      action: 'configure',
      keywords: keywordTags.value,
      cooldownSeconds: parseCooldown(),
      dispatchAgent: dispatchAgent.value || null,
      senderAllowList: parseLineSeparated(senderAllowList.value),
      senderDenyList: parseLineSeparated(senderDenyList.value),
    });
    applyStatus(status);
    statusMessage.value = status.message ?? 'Teams monitor settings saved.';
  } catch (saveError) {
    error.value = saveError instanceof Error ? saveError.message : String(saveError);
  } finally {
    isSaving.value = false;
  }
}

function updateDispatchAgent(value: string | number | bigint | Record<string, unknown> | null) {
  const nextValue = String(value ?? NO_AGENT_VALUE);
  dispatchAgent.value = nextValue === NO_AGENT_VALUE ? '' : nextValue;
}

onMounted(() => {
  void loadStatus();
});
</script>

<template>
  <section class="flex flex-col gap-4" role="form" aria-label="Teams monitor settings">
    <div class="flex flex-col gap-1">
      <h3 class="text-base font-semibold text-text-primary">Teams monitor</h3>
      <p class="text-sm text-text-secondary">
        Route Teams activity to an agent using keyword and sender filters.
      </p>
    </div>

    <p v-if="error" class="text-sm text-destructive" role="alert">{{ error }}</p>
    <p v-else-if="statusMessage" class="text-sm text-primary" role="status">{{ statusMessage }}</p>

    <div v-if="isLoading" class="rounded-md border border-border bg-muted/40 p-4 text-sm text-muted-foreground">
      Loading Teams monitor settings…
    </div>

    <template v-else>
      <div class="flex items-center justify-between gap-4 rounded-md border border-border bg-muted/40 p-4 max-sm:flex-col max-sm:items-stretch">
        <div class="flex flex-col gap-1">
          <Label for="teams-monitor-enabled" class="text-sm font-semibold">Enable monitor</Label>
          <p class="text-sm text-muted-foreground">
            Toggle Teams notifications monitoring without changing saved routing filters.
          </p>
        </div>
        <Switch
          id="teams-monitor-enabled"
          :model-value="enabled"
          :disabled="isToggling || isSaving"
          @update:model-value="handleToggle"
        />
      </div>

      <div class="grid gap-4">
        <div class="flex flex-col gap-2 rounded-md border border-border bg-muted/40 p-4">
          <Label for="teams-monitor-keywords" class="text-sm font-semibold">Keywords</Label>
          <Input
            id="teams-monitor-keywords"
            v-model="keywordsInput"
            placeholder="incident, sev0, urgent"
            :disabled="isSaving"
          />
          <div class="flex flex-wrap gap-2">
            <Badge v-for="keyword in keywordTags" :key="keyword" variant="secondary">
              {{ keyword }}
            </Badge>
            <span v-if="keywordTags.length === 0" class="text-sm text-muted-foreground">
              No keywords configured.
            </span>
          </div>
        </div>

        <div class="flex flex-col gap-2 rounded-md border border-border bg-muted/40 p-4">
          <Label for="teams-monitor-agent" class="text-sm font-semibold">Dispatch agent</Label>
          <Select :model-value="dispatchAgentValue" :disabled="isSaving" @update:model-value="updateDispatchAgent">
            <SelectTrigger id="teams-monitor-agent">
              <SelectValue placeholder="Select an agent" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem :value="NO_AGENT_VALUE">No agent</SelectItem>
              <SelectItem v-for="agent in agentNames" :key="agent" :value="agent">
                {{ agent }}
              </SelectItem>
            </SelectContent>
          </Select>
        </div>

          <div class="flex flex-col gap-2 rounded-md border border-border bg-muted/40 p-4">
            <Label for="teams-monitor-cooldown" class="text-sm font-semibold">Cooldown (seconds)</Label>
            <Input id="teams-monitor-cooldown" v-model="cooldownSeconds" type="number" min="0" step="1" :disabled="isSaving" />
          </div>

        <div class="grid gap-4 lg:grid-cols-2">
          <div class="flex flex-col gap-2 rounded-md border border-border bg-muted/40 p-4">
            <Label for="teams-monitor-allow" class="text-sm font-semibold">Sender allow list</Label>
            <Textarea
              id="teams-monitor-allow"
              v-model="senderAllowList"
              placeholder="name@contoso.com"
              :disabled="isSaving"
            />
            <p class="text-sm text-muted-foreground">One email address per line. Leave blank to allow all senders.</p>
          </div>

          <div class="flex flex-col gap-2 rounded-md border border-border bg-muted/40 p-4">
            <Label for="teams-monitor-deny" class="text-sm font-semibold">Sender deny list</Label>
            <Textarea
              id="teams-monitor-deny"
              v-model="senderDenyList"
              placeholder="spam@contoso.com"
              :disabled="isSaving"
            />
            <p class="text-sm text-muted-foreground">One email address per line. Deny rules override allow rules.</p>
          </div>
        </div>

        <div class="flex items-center justify-between gap-4 rounded-md border border-border bg-muted/40 p-4 max-sm:flex-col max-sm:items-stretch">
          <div class="flex flex-col gap-1">
            <span class="text-sm font-semibold text-text-primary">Connection status</span>
            <span class="text-sm text-muted-foreground">Current Teams monitor runtime connection state.</span>
          </div>
          <Badge variant="outline">{{ connectionStatusLabel }}</Badge>
        </div>
      </div>

      <div class="flex justify-end">
        <Button type="button" :disabled="isSaving || isToggling" @click="handleSave">
          {{ isSaving ? 'Saving…' : 'Save' }}
        </Button>
      </div>
    </template>
  </section>
</template>
