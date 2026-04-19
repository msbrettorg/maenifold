<script setup lang="ts">
import { Input } from '@/components/ui/input';
import { computed } from 'vue';
import { usePipelineStore } from '../stores/pipeline';
import DesignNodeConfigEdges from './DesignNodeConfigEdges.vue';
import DesignNodeConfigFields from './DesignNodeConfigFields.vue';

const props = defineProps<{
  selectedNodeId: string | null;
  selectedEdge: { source: string; target: string } | null;
}>();

const store = usePipelineStore();

const selectedDesignNode = computed(() =>
  props.selectedNodeId
    ? store.localDefinition.nodes.find((node) => node.id === props.selectedNodeId) ?? null
    : null,
);
const selectedDesignEdge = computed(() =>
  props.selectedEdge
    ? store.localDefinition.edges.find(
        (edge) =>
          edge.source === props.selectedEdge?.source &&
          edge.target === props.selectedEdge?.target,
      ) ?? null
    : null,
);
const inspectorMode = computed(() => {
  if (selectedDesignNode.value) return 'design-node';
  if (selectedDesignEdge.value) return 'design-edge';
  return !props.selectedNodeId && !props.selectedEdge
    ? 'design-overview'
    : 'fallback';
});

function updateMaxConcurrency(value: string | number): void {
  const nextValue = Number(value);
  store.maxConcurrency = Number.isFinite(nextValue)
    ? Math.min(32, Math.max(1, nextValue))
    : 1;
}
</script>

<template>
  <aside class="pipeline-inspector" aria-label="Node configuration">
    <section v-if="inspectorMode === 'design-overview'" class="pipeline-inspector__panel">
      <h2>Pipeline settings</h2>
      <label class="pipeline-inspector__field">
        <span>Max concurrency</span>
        <Input
          :model-value="store.maxConcurrency"
          type="number"
          min="1"
          max="32"
          @update:model-value="updateMaxConcurrency"
        />
      </label>
      <div class="pipeline-inspector__stats-grid">
        <div class="pipeline-inspector__stat-card">
          <span class="pipeline-inspector__stat-label">Nodes</span>
          <strong>{{ store.localDefinition.nodes.length }}</strong>
        </div>
        <div class="pipeline-inspector__stat-card">
          <span class="pipeline-inspector__stat-label">Edges</span>
          <strong>{{ store.localDefinition.edges.length }}</strong>
        </div>
      </div>
    </section>

    <DesignNodeConfigFields
      v-else-if="inspectorMode === 'design-node' && props.selectedNodeId"
      :selected-node-id="props.selectedNodeId"
    />

    <DesignNodeConfigEdges
      v-else-if="inspectorMode === 'design-edge' && props.selectedEdge"
      :selected-edge="props.selectedEdge"
    />

    <section v-else class="pipeline-inspector__panel pipeline-inspector__empty-state">
      <h2>Inspector</h2>
      <p>Select a node or edge to inspect</p>
    </section>
  </aside>
</template>
