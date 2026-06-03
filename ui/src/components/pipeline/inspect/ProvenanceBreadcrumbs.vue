<script setup lang="ts">
// Provenance breadcrumbs (T-600.7).
//
// Renders the lineage chain a pipeline node descends from — gate task, pipeline
// run, thinking session, memory note — from `node.provenance.detail.lineage*`.
//
// These lineage values are STABLE LINEAGE IDS, not navigable targets. The
// backend derives them as `lin-v1-<kind>-<sha256-prefix>` (crates/maenifold-core/
// src/lineage.rs:4-29) — an irreversible hash of the gate task id / pipeline
// session id / thinking session id / memory uri. They are NOT pipeline session
// ids and NOT memory uris, so there is no in-app destination to resolve them to.
// Every crumb is therefore inert context: we render the lineage but do NOT
// fabricate a clickable destination that would dead-end on a lookup miss.
import { computed } from 'vue';
import { GitBranch, Workflow, FileText, Brain } from '@lucide/vue';
import { type ViewNode } from '../../../lib/pipeline-types';

const props = defineProps<{ node: ViewNode | null }>();

type CrumbKind = 'gate' | 'pipeline' | 'thinking' | 'memory';
interface Crumb {
  kind: CrumbKind;
  label: string;
  value: string;
}

const crumbs = computed<Crumb[]>(() => {
  const detail = props.node?.provenance?.detail;
  if (!detail) return [];
  const list: Crumb[] = [];
  if (detail.lineageGateTaskId) {
    list.push({ kind: 'gate', label: 'Gate task', value: detail.lineageGateTaskId });
  }
  if (detail.lineagePipelineRunId) {
    list.push({ kind: 'pipeline', label: 'Pipeline run', value: detail.lineagePipelineRunId });
  }
  if (detail.lineageThinkingSessionId) {
    list.push({ kind: 'thinking', label: 'Thinking session', value: detail.lineageThinkingSessionId });
  }
  if (detail.lineageMemoryNoteId) {
    list.push({ kind: 'memory', label: 'Memory note', value: detail.lineageMemoryNoteId });
  }
  return list;
});
</script>

<template>
  <nav v-if="crumbs.length" class="prov" aria-label="Node provenance">
    <span class="prov-title">Lineage</span>
    <ol class="prov-trail">
      <li v-for="(crumb, idx) in crumbs" :key="crumb.kind + crumb.value" class="prov-item">
        <span v-if="idx > 0" class="prov-sep" aria-hidden="true">›</span>
        <span class="prov-crumb" :title="crumb.value">
          <GitBranch v-if="crumb.kind === 'gate'" :size="12" :stroke-width="2" />
          <Workflow v-else-if="crumb.kind === 'pipeline'" :size="12" :stroke-width="2" />
          <Brain v-else-if="crumb.kind === 'thinking'" :size="12" :stroke-width="2" />
          <FileText v-else-if="crumb.kind === 'memory'" :size="12" :stroke-width="2" />
          <span class="prov-label">{{ crumb.label }}</span>
          <span class="prov-value">{{ crumb.value }}</span>
        </span>
      </li>
    </ol>
  </nav>
</template>

<style scoped>
.prov {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.prov-title {
  font-size: 0.66rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.03em;
  color: var(--text-muted);
}

.prov-trail {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.2rem;
  margin: 0;
  padding: 0;
  list-style: none;
}

.prov-item {
  display: inline-flex;
  align-items: center;
  gap: 0.2rem;
  min-width: 0;
}

.prov-sep {
  color: var(--text-muted);
  font-size: 0.8rem;
}

.prov-crumb {
  display: inline-flex;
  align-items: center;
  gap: 0.3rem;
  max-width: 16rem;
  padding: 0.18rem 0.4rem;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  background: var(--bg-main);
  color: var(--text-secondary);
  font-size: 0.7rem;
  font-family: inherit;
}

.prov-label {
  flex-shrink: 0;
  text-transform: uppercase;
  font-size: 0.6rem;
  letter-spacing: 0.02em;
  color: var(--text-muted);
}

.prov-value {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
}
</style>
