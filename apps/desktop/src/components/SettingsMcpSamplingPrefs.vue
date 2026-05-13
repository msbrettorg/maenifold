<script setup lang="ts">
import { computed, ref } from 'vue';
import { Badge } from '@/components/ui/badge';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Switch } from '@/components/ui/switch';
import type { McpServerEntry } from '@/lib/types';
import { useConfigStore } from '@/stores/config';

const configStore = useConfigStore();
const errorByServer = ref<Record<string, string>>({});
const savingByServer = ref<Record<string, boolean>>({});

const servers = computed<McpServerEntry[]>(() => configStore.config.mcpServers ?? []);

function transportLabel(type: McpServerEntry['type']): string {
  return type === 'stdio' ? 'Stdio' : 'Http';
}

function isAlwaysAllowed(serverName: string): boolean {
  return configStore.config.samplingPreferences?.perServer?.[serverName]?.alwaysAllow ?? false;
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

async function setAlwaysAllowed(serverName: string, alwaysAllow: boolean): Promise<void> {
  savingByServer.value = { ...savingByServer.value, [serverName]: true };
  const { [serverName]: _oldError, ...remainingErrors } = errorByServer.value;
  errorByServer.value = remainingErrors;

  try {
    await configStore.setSamplingPreference(serverName, { alwaysAllow });
  } catch (error) {
    errorByServer.value = { ...errorByServer.value, [serverName]: errorMessage(error) };
  } finally {
    savingByServer.value = { ...savingByServer.value, [serverName]: false };
  }
}
</script>

<template>
  <section class="flex flex-col gap-4" role="region" aria-label="Sampling preferences">
    <Card>
      <CardHeader>
        <CardTitle class="text-base">Sampling preferences</CardTitle>
        <CardDescription>
          Choose which MCP servers may always allow sampling requests without prompting.
        </CardDescription>
      </CardHeader>

      <CardContent class="flex flex-col gap-4">
        <div
          v-if="servers.length === 0"
          class="rounded-md bg-muted/40 p-4 text-sm text-muted-foreground"
        >
          No MCP servers configured.
        </div>

        <div v-else class="grid gap-3">
          <Card
            v-for="server in servers"
            :key="`${server.type}-${server.name}`"
            data-testid="mcp-sampling-pref-card"
          >
            <CardHeader>
              <div class="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
                <div class="flex min-w-0 flex-col gap-2">
                  <div class="flex flex-wrap items-center gap-2">
                    <CardTitle class="text-base font-semibold">{{ server.name }}</CardTitle>
                    <Badge variant="secondary">{{ transportLabel(server.type) }}</Badge>
                  </div>
                  <CardDescription>
                    Preference is preserved if the server is removed and later re-added.
                  </CardDescription>
                </div>

                <div class="flex shrink-0 items-center gap-3">
                  <span class="text-sm text-muted-foreground">Always allow sampling</span>
                  <Switch
                    :aria-label="`Always allow sampling for ${server.name}`"
                    :checked="isAlwaysAllowed(server.name)"
                    :model-value="isAlwaysAllowed(server.name)"
                    :disabled="savingByServer[server.name] ?? false"
                    @update:checked="(value: boolean) => setAlwaysAllowed(server.name, value)"
                    @update:model-value="(value: boolean) => setAlwaysAllowed(server.name, value)"
                  />
                </div>
              </div>
              <p
                v-if="errorByServer[server.name]"
                class="text-sm text-destructive"
                role="alert"
              >
                {{ errorByServer[server.name] }}
              </p>
            </CardHeader>
          </Card>
        </div>
      </CardContent>
    </Card>
  </section>
</template>
