<script setup lang="ts">
// SemanticZoom content renderer: a node's raw output, rendered by MIME SNIFFING
// the content (never by trusting a node field):
//   - parse-able JSON object/array → pretty-printed in a <pre>
//   - Markdown markers → rendered via the shared `marked` dependency
//   - otherwise → plain text in a <pre>
// The sniffed format is surfaced as a small badge so the heuristic is honest.
import { computed } from 'vue';
import { marked } from 'marked';
import { Braces, FileText, Hash } from '@lucide/vue';
import { prettyJson, sniffContentFormat, tryParseJsonStructure } from './zoomLogic';

const props = defineProps<{ output: string }>();

const format = computed(() => sniffContentFormat(props.output));

const prettyJsonText = computed(() => {
  const parsed = tryParseJsonStructure(props.output);
  return parsed.ok ? prettyJson(parsed.value) : props.output;
});

const markdownHtml = computed(() => marked.parse(props.output, { async: false }) as string);

const FORMAT_META = {
  json: { label: 'JSON', icon: Braces },
  markdown: { label: 'Markdown', icon: Hash },
  text: { label: 'Plain text', icon: FileText },
} as const;
</script>

<template>
  <div class="zout">
    <span class="zout-badge">
      <component :is="FORMAT_META[format].icon" :size="12" :stroke-width="2" />
      {{ FORMAT_META[format].label }}
    </span>

    <pre v-if="format === 'json'" class="zout-pre">{{ prettyJsonText }}</pre>
    <!-- eslint-disable-next-line vue/no-v-html -- sniffed markdown from our own MCP backend -->
    <div v-else-if="format === 'markdown'" class="zout-md" v-html="markdownHtml" />
    <pre v-else class="zout-pre zout-text">{{ props.output }}</pre>
  </div>
</template>

<style scoped>
.zout {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.zout-badge {
  align-self: flex-start;
  display: inline-flex;
  align-items: center;
  gap: 0.3rem;
  padding: 0.12rem 0.45rem;
  border-radius: 999px;
  background: color-mix(in srgb, var(--text-accent) 14%, transparent);
  color: var(--text-accent);
  font-size: 0.68rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.02em;
}

.zout-pre {
  margin: 0;
  padding: 0.65rem 0.75rem;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: color-mix(in srgb, var(--bg-main) 92%, var(--bg-sidebar));
  color: var(--text-primary);
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 0.78rem;
  line-height: 1.5;
  white-space: pre-wrap;
  word-break: break-word;
}

.zout-md {
  padding: 0.25rem 0.1rem;
  color: var(--text-primary);
  font-size: 0.84rem;
  line-height: 1.55;
}

.zout-md :deep(h1),
.zout-md :deep(h2),
.zout-md :deep(h3) {
  margin: 0.8rem 0 0.4rem;
  font-size: 0.95rem;
}

.zout-md :deep(pre) {
  padding: 0.55rem 0.7rem;
  border: 1px solid var(--border-color);
  border-radius: 7px;
  background: color-mix(in srgb, var(--bg-main) 92%, var(--bg-sidebar));
  overflow-x: auto;
  font-size: 0.76rem;
}

.zout-md :deep(code) {
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 0.78rem;
}

.zout-md :deep(a) {
  color: var(--text-accent);
}
</style>
