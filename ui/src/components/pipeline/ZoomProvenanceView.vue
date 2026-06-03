<script setup lang="ts">
// SemanticZoom content renderer: a tool node's provenance — the resolved tool
// name, its resolved arguments (pretty JSON) and the response summary.
import { computed } from 'vue';
import { Wrench } from '@lucide/vue';
import { prettyJson } from './zoomLogic';
import type { ToolProvenance } from '../../lib/pipeline-types';

const props = defineProps<{ provenance: ToolProvenance }>();

const argsText = computed(() =>
  props.provenance.resolvedArgs ? prettyJson(props.provenance.resolvedArgs) : null
);
</script>

<template>
  <div class="zprov">
    <header class="zprov-tool">
      <Wrench :size="15" :stroke-width="2" />
      <span class="zprov-name">{{ props.provenance.toolName }}</span>
      <span class="zprov-zero" title="Tool node — runs with zero LLM tokens">⚡ 0 tokens</span>
    </header>

    <section class="zprov-block">
      <h4 class="zprov-label">Resolved arguments</h4>
      <pre v-if="argsText" class="zprov-pre">{{ argsText }}</pre>
      <p v-else class="zprov-empty">No resolved arguments.</p>
    </section>

    <section class="zprov-block">
      <h4 class="zprov-label">Response summary</h4>
      <pre v-if="props.provenance.responseSummary" class="zprov-pre">{{ props.provenance.responseSummary }}</pre>
      <p v-else class="zprov-empty">No response summary.</p>
    </section>
  </div>
</template>

<style scoped>
.zprov {
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
}

.zprov-tool {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
  color: var(--node-accent-tool);
}

.zprov-name {
  font-size: 0.9rem;
  font-weight: 700;
  color: var(--text-primary);
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
}

.zprov-zero {
  display: inline-flex;
  align-items: center;
  gap: 0.2rem;
  padding: 0.1rem 0.4rem;
  border-radius: 999px;
  background: color-mix(in srgb, var(--node-accent-tool) 22%, transparent);
  color: color-mix(in srgb, var(--node-accent-tool) 80%, var(--text-primary));
  font-size: 0.66rem;
  font-weight: 700;
}

.zprov-block {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.zprov-label {
  margin: 0;
  font-size: 0.72rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.03em;
  color: var(--text-accent);
}

.zprov-pre {
  margin: 0;
  padding: 0.6rem 0.7rem;
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

.zprov-empty {
  margin: 0;
  font-size: 0.78rem;
  color: var(--text-muted);
}
</style>
