<script setup lang="ts">
import { computed } from 'vue';
import { X } from 'lucide-vue-next';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import type { ViewNode } from '../lib/pipeline-types';

const props = defineProps<{ node: ViewNode }>();
const emit = defineEmits<{ (e: 'close'): void }>();

type Detail = { label: string; value: string };

function hasContent(value: unknown): boolean {
  if (value == null) return false;
  if (typeof value === 'string') return value.trim().length > 0;
  if (Array.isArray(value)) return value.length > 0;
  return true;
}

function formatValue(value: unknown): string {
  if (typeof value === 'string') return value;
  if (typeof value === 'number' || typeof value === 'boolean') return String(value);
  return JSON.stringify(value, null, 2);
}

function addDetail(details: Detail[], label: string, value: unknown): void {
  if (!hasContent(value)) return;
  details.push({ label, value: formatValue(value) });
}

function setConfig(config: Record<string, unknown>, key: string, value: unknown): void {
  if (hasContent(value)) config[key] = value;
}

const nodeType = computed(() => props.node.type ?? 'workflow');
const titleLabel = computed(() => props.node.displayLabel ?? props.node.nodeId);

const details = computed<Detail[]>(() => {
  const rows: Detail[] = [];
  addDetail(rows, 'Node ID', props.node.nodeId);
  addDetail(rows, 'Type', nodeType.value);
  addDetail(rows, 'Status', props.node.status);
  addDetail(rows, 'Attempts', props.node.attempts);
  addDetail(rows, 'Session', props.node.acpSessionId ?? props.node.sessionId);
  return rows;
});

const promptText = computed(() => {
  if ('prompt' in props.node && hasContent(props.node.prompt)) return props.node.prompt ?? '';
  if ('assembled_prompt' in props.node && hasContent(props.node.assembled_prompt)) return props.node.assembled_prompt ?? '';
  return '';
});

const configText = computed(() => {
  const config: Record<string, unknown> = {};
  const node = props.node;

  if (node.type === 'tool') {
    setConfig(config, 'toolName', node.toolName);
    setConfig(config, 'toolArgs', node.toolArgs);
    setConfig(config, 'toolProvenance', node.toolProvenance);
  } else if (node.type === 'prompt') {
    setConfig(config, 'agent', node.agent);
    setConfig(config, 'toolMode', node.toolMode);
    setConfig(config, 'sessionRef', node.sessionRef);
  } else if (node.type === 'sequentialThinking') {
    setConfig(config, 'agent', node.agent);
    setConfig(config, 'topic', node.topic);
    setConfig(config, 'totalThoughts', node.totalThoughts);
    setConfig(config, 'analysisType', node.analysisType);
  } else {
    setConfig(config, 'workflowId', node.workflowId);
    setConfig(config, 'acpClient', node.acpClient);
    setConfig(config, 'workflowSteps', node.workflow_steps);
  }

  if ('model' in node) setConfig(config, 'model', node.model);
  if ('thoughtLevel' in node) setConfig(config, 'thoughtLevel', node.thoughtLevel);
  if ('allowedTools' in node) setConfig(config, 'allowedTools', node.allowedTools);
  if ('maxTurns' in node) setConfig(config, 'maxTurns', node.maxTurns);
  if ('cwd' in node) setConfig(config, 'cwd', node.cwd);
  if ('timeout' in node) setConfig(config, 'timeout', node.timeout);
  if ('agentConfig' in node) setConfig(config, 'agentConfig', node.agentConfig);
  if ('adopt' in node) setConfig(config, 'adopt', node.adopt);
  if ('requiresRole' in node) setConfig(config, 'requiresRole', node.requiresRole);

  return Object.keys(config).length > 0 ? JSON.stringify(config, null, 2) : '';
});

const outputPreview = computed(() => {
  const node = props.node;
  const output = hasContent(node.output) ? node.output ?? '' : '';
  if (output) return output.length > 1200 ? `${output.slice(0, 1200).trimEnd()}...` : output;
  if (node.type === 'tool' && hasContent(node.toolProvenance?.responseSummary)) {
    return node.toolProvenance?.responseSummary ?? '';
  }
  return '';
});
</script>

<template>
  <aside class="relative z-10 flex h-full w-80 shrink-0 bg-surface-high" aria-label="Node inspector">
    <Card class="flex h-full w-full gap-0 rounded-none border-y-0 border-r-0 bg-surface text-text-primary shadow-fluent-16">
      <CardHeader class="flex shrink-0 flex-row items-start justify-between gap-3 px-4 py-3">
        <div class="min-w-0 space-y-1">
          <CardTitle class="truncate text-title font-semibold">{{ titleLabel }}</CardTitle>
          <Badge variant="outline" class="font-normal">{{ nodeType }}</Badge>
        </div>
        <Button type="button" variant="ghost" size="icon-sm" aria-label="Close node inspector" @click="emit('close')">
          <X class="size-4" aria-hidden="true" />
        </Button>
      </CardHeader>

      <CardContent class="min-h-0 flex-1 p-0">
        <ScrollArea class="h-full px-4 pb-4">
          <dl class="grid gap-2 text-sm">
            <div v-for="detail in details" :key="detail.label" class="grid gap-0.5">
              <dt class="text-xs font-medium uppercase tracking-wide text-muted-foreground">{{ detail.label }}</dt>
              <dd class="break-words font-mono text-xs text-text-primary">{{ detail.value }}</dd>
            </div>
          </dl>

          <section v-if="promptText" class="mt-4 grid gap-2">
            <h3 class="text-sm font-semibold text-text-primary">Prompt</h3>
            <pre class="max-h-56 overflow-auto whitespace-pre-wrap break-words rounded-lg bg-muted/50 p-3 font-mono text-xs text-muted-foreground">{{ promptText }}</pre>
          </section>

          <section v-if="configText" class="mt-4 grid gap-2">
            <h3 class="text-sm font-semibold text-text-primary">Config</h3>
            <pre class="max-h-64 overflow-auto whitespace-pre-wrap break-words rounded-lg bg-muted/50 p-3 font-mono text-xs text-muted-foreground">{{ configText }}</pre>
          </section>

          <section v-if="outputPreview" class="mt-4 grid gap-2">
            <h3 class="text-sm font-semibold text-text-primary">Output preview</h3>
            <pre class="max-h-64 overflow-auto whitespace-pre-wrap break-words rounded-lg bg-muted/50 p-3 font-mono text-xs text-muted-foreground">{{ outputPreview }}</pre>
          </section>
        </ScrollArea>
      </CardContent>
    </Card>
  </aside>
</template>
