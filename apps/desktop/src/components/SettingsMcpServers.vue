<script setup lang="ts">
import { computed, ref } from 'vue';
import { Edit, Plus, Trash2 } from 'lucide-vue-next';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import type { AgentsConfig, McpServerEntry } from '@/lib/types';
import { useConfirmDialogStore } from '@/stores/confirm-dialog';
import { useConfigStore } from '@/stores/config';
import SettingsMcpServerEditor from './SettingsMcpServerEditor.vue';

type EditableServer = McpServerEntry & {
  env?: Record<string, string>;
  timeout?: number;
};

const configStore = useConfigStore();
const confirmDialogStore = useConfirmDialogStore();
const editorOpen = ref(false);
const editingIndex = ref<number | null>(null);
const isSaving = ref(false);
const saveError = ref<string | null>(null);

const servers = computed<EditableServer[]>(() => (configStore.config.mcpServers ?? []) as EditableServer[]);
const existingNames = computed(() => servers.value.map((server) => server.name));
const editingServer = computed(() =>
  editingIndex.value === null ? null : servers.value[editingIndex.value] ?? null
);

function openAdd(): void {
  editingIndex.value = null;
  saveError.value = null;
  editorOpen.value = true;
}

function openEdit(index: number): void {
  editingIndex.value = index;
  saveError.value = null;
  editorOpen.value = true;
}

function transportLabel(type: McpServerEntry['type']): string {
  return type === 'stdio' ? 'Stdio' : 'Http';
}

function argSummary(server: EditableServer): string {
  if (server.type !== 'stdio' || !server.args?.length) return 'No args';
  return server.args.join(' ');
}

function envCount(server: EditableServer): number {
  return server.type === 'stdio' && server.env ? Object.keys(server.env).length : 0;
}

async function persistServers(nextServers: EditableServer[]): Promise<void> {
  const writer = configStore as unknown as {
    writeConfig?: (updates: Partial<AgentsConfig>) => Promise<void>;
  };
  if (writer.writeConfig) {
    await writer.writeConfig({ mcpServers: nextServers });
    return;
  }
  configStore.config = { ...configStore.config, mcpServers: nextServers };
  await configStore.save();
}

async function removeServer(index: number): Promise<void> {
  const server = servers.value[index];
  if (!server) return;
  const confirmed = await confirmDialogStore.confirm(
    'Remove MCP server?',
    `Remove "${server.name}" from maenifold.json?`,
  );
  if (!confirmed) return;
  isSaving.value = true;
  saveError.value = null;
  try {
    await persistServers(servers.value.filter((_, currentIndex) => currentIndex !== index));
  } catch (error) {
    saveError.value = error instanceof Error ? error.message : String(error);
  } finally {
    isSaving.value = false;
  }
}

async function saveServer(server: EditableServer): Promise<void> {
  const nextServers = [...servers.value];
  if (editingIndex.value === null) {
    nextServers.push(server);
  } else {
    nextServers[editingIndex.value] = server;
  }
  isSaving.value = true;
  saveError.value = null;
  try {
    await persistServers(nextServers);
    editorOpen.value = false;
    editingIndex.value = null;
  } catch (error) {
    saveError.value = error instanceof Error ? error.message : String(error);
  } finally {
    isSaving.value = false;
  }
}
</script>

<template>
  <section class="flex flex-col gap-4" role="region" aria-label="MCP servers">
    <Card>
      <CardHeader>
        <div class="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
          <div class="flex flex-col gap-1">
            <CardTitle class="text-base">MCP servers</CardTitle>
            <CardDescription>
              View and edit Stdio and Http servers stored in maenifold.json.
            </CardDescription>
          </div>
          <Button type="button" size="sm" class="shrink-0" @click="openAdd">
            <Plus :size="14" class="mr-1" /> Add server
          </Button>
        </div>
      </CardHeader>
      <CardContent class="flex flex-col gap-4">
        <p v-if="saveError" class="text-sm text-destructive" role="alert">{{ saveError }}</p>

        <div
          v-if="servers.length === 0"
          class="flex flex-col items-start gap-3 rounded-md bg-muted/40 p-4 text-sm text-muted-foreground"
        >
          <p>No MCP servers configured yet. Click 'Add server' to add one.</p>
          <Button type="button" @click="openAdd">
            <Plus :size="14" class="mr-1" /> Add server
          </Button>
        </div>

        <div v-else class="grid gap-3">
          <Card v-for="(server, index) in servers" :key="`${server.type}-${server.name}`">
            <CardHeader>
              <div class="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
                <div class="flex min-w-0 flex-col gap-2">
                  <div class="flex flex-wrap items-center gap-2">
                    <CardTitle class="text-base font-semibold">{{ server.name }}</CardTitle>
                    <Badge variant="secondary">{{ transportLabel(server.type) }}</Badge>
                    <Badge v-if="server.type === 'stdio'" variant="outline">
                      {{ envCount(server) }} env vars
                    </Badge>
                  </div>
                  <CardDescription v-if="server.type === 'stdio'" class="break-all">
                    {{ server.command }} · {{ argSummary(server) }}
                  </CardDescription>
                  <CardDescription v-else class="break-all">{{ server.url }}</CardDescription>
                </div>
                <div class="flex shrink-0 gap-2">
                  <Button type="button" variant="outline" size="sm" @click="openEdit(index)">
                    <Edit :size="14" class="mr-1" /> Edit
                  </Button>
                  <Button type="button" variant="destructive" size="sm" :disabled="isSaving" @click="removeServer(index)">
                    <Trash2 :size="14" class="mr-1" /> Remove
                  </Button>
                </div>
              </div>
            </CardHeader>
          </Card>
        </div>
      </CardContent>
    </Card>

    <SettingsMcpServerEditor
      :open="editorOpen"
      :server="editingServer"
      :existing-names="existingNames"
      @update:open="(open) => (editorOpen = open)"
      @save="saveServer"
    />
  </section>
</template>
