<script setup lang="ts">
import { reactive } from 'vue'
import { Eye, EyeOff } from 'lucide-vue-next'
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from '@/components/ui/accordion'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'

interface EnvVarRow {
  key: string;
  value: string;
}

interface McpServerFormRow {
  type: 'stdio' | 'http';
  name: string;
  commandOrUrl: string;
  args: string;
}

const {
  form,
  onAddEnvRow,
  onRemoveEnvRow,
  onAddMcpServerRow,
  onRemoveMcpServerRow,
} = defineProps<{
  form: {
    env: EnvVarRow[];
    mcpServers: McpServerFormRow[];
  };
  onAddEnvRow: () => void;
  onRemoveEnvRow: (index: number) => void;
  onAddMcpServerRow: () => void;
  onRemoveMcpServerRow: (index: number) => void;
}>();

const revealedEnvValues = reactive<Record<string, boolean>>({});

function isSecretEnvKey(key: string) {
  return /TOKEN|KEY|SECRET|PASSWORD/i.test(key);
}

function getEnvRevealKey(entry: EnvVarRow) {
  return entry.key;
}

function isEnvValueRevealed(entry: EnvVarRow) {
  return Boolean(revealedEnvValues[getEnvRevealKey(entry)]);
}

function getEnvValueInputType(entry: EnvVarRow) {
  if (!isSecretEnvKey(entry.key)) {
    return 'text';
  }

  return isEnvValueRevealed(entry) ? 'text' : 'password';
}

function toggleEnvValueReveal(entry: EnvVarRow) {
  const revealKey = getEnvRevealKey(entry);
  revealedEnvValues[revealKey] = !revealedEnvValues[revealKey];
}

function removeEnvRow(index: number) {
  const entry = form.env[index];
  if (entry) {
    delete revealedEnvValues[getEnvRevealKey(entry)];
  }

  onRemoveEnvRow(index);
}

function formatServerCount(count: number) {
  return `${count} MCP server${count === 1 ? '' : 's'}`;
}

function formatEnvCount(count: number) {
  return `${count} env var${count === 1 ? '' : 's'}`;
}
</script>

<template>
  <Accordion type="multiple" :default-value="['env', 'mcp']">
    <AccordionItem value="env" class="flex flex-col gap-3 p-[0.9rem] border border-border-subtle rounded bg-bg-app">
      <AccordionTrigger>
        <div>
          <h5 class="m-0 text-sm font-medium text-text-secondary">Environment variables</h5>
          <p class="m-0 text-text-secondary">Optional key/value pairs passed to this agent process.</p>
        </div>
        <span class="text-text-secondary text-[0.75rem] px-2 py-[0.15rem] rounded bg-bg-elevated whitespace-nowrap">{{ formatEnvCount(form.env.length) }}</span>
      </AccordionTrigger>
      <AccordionContent>
        <div class="flex flex-col gap-3">
          <div v-if="form.env.length === 0" class="text-text-secondary text-[0.75rem]">No environment variables configured.</div>
          <div v-for="(entry, index) in form.env" :key="`env-${index}`" class="flex justify-between gap-3 items-end max-sm:flex-col max-sm:items-stretch">
            <Input v-model="entry.key" type="text" placeholder="KEY" class="max-w-[14rem] max-sm:max-w-none" />
            <div class="flex flex-1 gap-2 items-center w-full">
              <Input
                v-model="entry.value"
                :type="getEnvValueInputType(entry)"
                placeholder="value"
              />
              <Button
                v-if="isSecretEnvKey(entry.key)"
                variant="ghost"
                size="icon"
                class="w-8 h-8 p-0 shrink-0 border border-border-subtle rounded bg-transparent text-inherit cursor-pointer"
                type="button"
                :aria-label="isEnvValueRevealed(entry) ? 'Hide variable value' : 'Reveal variable value'"
                :aria-pressed="isEnvValueRevealed(entry)"
                @click="toggleEnvValueReveal(entry)"
              >
                <EyeOff v-if="isEnvValueRevealed(entry)" class="w-4 h-4" aria-hidden="true" />
                <Eye v-else class="w-4 h-4" aria-hidden="true" />
              </Button>
            </div>
            <Button variant="ghost" size="icon" class="w-8 h-8 p-0 self-end border border-border-subtle rounded bg-transparent text-inherit cursor-pointer text-[0.9375rem] leading-none max-sm:self-start" type="button" aria-label="Remove variable" @click="removeEnvRow(index)">×</Button>
          </div>
          <Button variant="outline" class="px-[0.9rem] py-2 border border-border-subtle rounded bg-transparent text-inherit cursor-pointer" type="button" @click="onAddEnvRow">+ Add Variable</Button>
        </div>
      </AccordionContent>
    </AccordionItem>

    <AccordionItem value="mcp" class="flex flex-col gap-3 p-[0.9rem] border border-border-subtle rounded bg-bg-app">
      <AccordionTrigger>
        <div>
          <h5 class="m-0 text-sm font-medium text-text-secondary">Per-agent MCP servers</h5>
          <p class="m-0 text-text-secondary">Optional MCP servers available only to this agent.</p>
        </div>
        <span class="text-text-secondary text-[0.75rem] px-2 py-[0.15rem] rounded bg-bg-elevated whitespace-nowrap">{{ formatServerCount(form.mcpServers.length) }}</span>
      </AccordionTrigger>
      <AccordionContent>
        <div class="flex flex-col gap-3">
          <div v-if="form.mcpServers.length === 0" class="text-text-secondary text-[0.75rem]">No per-agent MCP servers configured.</div>
          <div v-for="(server, index) in form.mcpServers" :key="`mcp-${index}`" class="flex flex-col gap-3 p-3 border border-border-subtle rounded bg-bg-elevated">
            <div class="flex justify-between gap-3 items-center">
              <strong>Server {{ index + 1 }}</strong>
              <Button variant="ghost" size="icon" class="w-8 h-8 p-0 self-end border border-border-subtle rounded bg-transparent text-inherit cursor-pointer text-[0.9375rem] leading-none max-sm:self-start" type="button" aria-label="Remove MCP server" @click="onRemoveMcpServerRow(index)">
                ×
              </Button>
            </div>

            <div class="flex flex-col gap-3">
              <div class="flex flex-col gap-[0.45rem]">
                <Label>Type</Label>
                <Select :model-value="server.type" @update:model-value="(val) => server.type = val as 'stdio' | 'http'">
                  <SelectTrigger class="w-full">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="stdio">stdio</SelectItem>
                    <SelectItem value="http">http</SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <div class="flex flex-col gap-[0.45rem]">
                <Label>Name</Label>
                <Input v-model="server.name" type="text" placeholder="server-name" />
              </div>
              <div class="flex flex-col gap-[0.45rem] w-full">
                <Label>{{ server.type === 'http' ? 'URL' : 'Command' }}</Label>
                <Input
                  v-model="server.commandOrUrl"
                  type="text"
                  :placeholder="server.type === 'http' ? 'https://example.test/mcp' : 'npx @scope/server'"
                />
              </div>
              <div v-if="server.type === 'stdio'" class="flex flex-col gap-[0.45rem] w-full">
                <Label>Args</Label>
                <Input v-model="server.args" type="text" placeholder="--flag value" />
                <small class="text-[0.75rem]">Space-separated arguments.</small>
              </div>
            </div>
          </div>

          <Button variant="outline" class="px-[0.9rem] py-2 border border-border-subtle rounded bg-transparent text-inherit cursor-pointer" type="button" @click="onAddMcpServerRow">+ Add MCP Server</Button>
        </div>
      </AccordionContent>
    </AccordionItem>
  </Accordion>
</template>
