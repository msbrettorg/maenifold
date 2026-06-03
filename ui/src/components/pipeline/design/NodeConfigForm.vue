<script setup lang="ts">
// Design-mode node config form (T-600.4, refactored in T-600.5). Teleported into
// `#pipeline-overlay`. Opens when the design controller has an `editingNodeId`.
// All field rendering + build/validation now lives in the shared, stateless
// NodeFormCore (the single field source of truth); this wrapper owns ONLY the
// design-store commit semantics: it seeds the core from `design.localDefinition`
// and commits the emitted node via `design.replaceNode`.
import { computed } from 'vue';
import { usePipelineDesignStore } from '../../../stores/pipelineDesign';
import { useDesignController } from './useDesignController';
import NodeFormCore from '../surgery/NodeFormCore.vue';
import type { PipelineNode, PipelineNodeType } from '../../../lib/pipeline-types';

const design = usePipelineDesignStore();
const controller = useDesignController();

const open = computed(() => controller.editingNodeId.value !== null);

const editingNode = computed<PipelineNode | null>(() => {
  const id = controller.editingNodeId.value;
  if (!id) return null;
  return design.localDefinition?.nodes.find((node) => node.id === id) ?? null;
});

const headingType = computed<string>(() => {
  const type = (editingNode.value?.type ?? 'workflow') as PipelineNodeType;
  return type === 'sequentialThinking' ? 'thinking' : type;
});

function onSubmit(node: PipelineNode): void {
  const id = controller.editingNodeId.value;
  if (!id) return;
  design.replaceNode(id, node);
  controller.closeNodeForm();
}

function cancel(): void {
  controller.closeNodeForm();
}
</script>

<template>
  <Teleport defer to="#pipeline-overlay">
    <div v-if="open && editingNode" class="form-backdrop" @click.self="cancel">
      <section class="form-panel" role="dialog" aria-label="Edit node">
        <header class="form-head">
          <h2>Edit {{ headingType }} node</h2>
          <code class="form-id">{{ controller.editingNodeId.value }}</code>
        </header>

        <NodeFormCore
          :node="(editingNode as unknown as Record<string, unknown>)"
          submit-label="Apply"
          @submit="onSubmit"
          @cancel="cancel"
        />
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.form-backdrop {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: color-mix(in srgb, var(--bg-main) 30%, transparent);
  z-index: 20;
}

.form-panel {
  display: flex;
  flex-direction: column;
  width: min(540px, 92%);
  max-height: 88%;
  border: 1px solid var(--border-color);
  border-radius: 10px;
  background: var(--bg-main);
  box-shadow: 0 14px 40px rgba(0, 0, 0, 0.32);
  overflow: hidden;
}

.form-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  padding: 0.7rem 0.9rem;
  border-bottom: 1px solid var(--border-color);
}

.form-head h2 {
  margin: 0;
  font-size: 0.92rem;
  color: var(--text-primary);
  text-transform: capitalize;
}

.form-id {
  font-size: 0.7rem;
  color: var(--text-muted);
}
</style>
