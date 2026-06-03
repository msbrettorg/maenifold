<script setup lang="ts">
// Node inspector pane (T-600.6). Teleported into the shell's `#pipeline-inspector`
// mount target. Driven by the canvas's current single selection
// (`controller.selectedNodeId`), resolved against the store's `selectedNodes`.
// Shows identity / status / session / prompt / config / output preview, and a
// "zoom" affordance that opens the SemanticZoom overlay for the node.
import { computed, onUnmounted, watch } from 'vue';
import { storeToRefs } from 'pinia';
import { Maximize2, MousePointerClick, Octagon, RotateCcw } from '@lucide/vue';
import { usePipelinesStore } from '../../stores/pipelines';
import { usePipelineInspectionStore } from '../../stores/pipelineInspection';
import { useCanvasController } from './useCanvasController';
import { availableSurfaces, nodeConfigEntries, type ConfigEntry } from './zoomLogic';
import { useSurgeryActions } from './surgery/useSurgeryActions';
import { canRestartNode, canAbandonNode } from '../../lib/surgeryEnablement';
import SessionInspectorPanel from './inspect/SessionInspectorPanel.vue';
import ProvenanceBreadcrumbs from './inspect/ProvenanceBreadcrumbs.vue';
import {
  isWorkflowViewNode,
  viewNodeDisplayLabel,
  viewNodeType,
  type ViewNode,
} from '../../lib/pipeline-types';
import { isInspectableNode } from '../../lib/sessionResolve';

const store = usePipelinesStore();
const { selectedNodes, selectedView } = storeToRefs(store);
const controller = useCanvasController();
const inspection = usePipelineInspectionStore();

const node = computed<ViewNode | null>(() => {
  const id = controller.selectedNodeId.value;
  if (!id) return null;
  return selectedNodes.value.find((candidate) => candidate.nodeId === id) ?? null;
});

// The node the LIVE observer should attach to. The zoom overlay opens for the
// selected node, so an open zoom takes precedence (scope #4: opening zoom
// auto-reconnects the live stream); otherwise the selected node. Resolved
// against the full view node set so a zoom target that isn't in the (single)
// selection still resolves.
const inspectionTarget = computed<ViewNode | null>(() => {
  const id = controller.zoomNodeId.value ?? controller.selectedNodeId.value;
  if (!id) return null;
  const fromView = selectedView.value?.nodes.find((candidate) => candidate.nodeId === id);
  if (fromView) return fromView;
  return selectedNodes.value.find((candidate) => candidate.nodeId === id) ?? null;
});

// NodeInspectorPane is the always-mounted owner of the inspection lifecycle: it
// drives grace tracking from the live view and the single live attachment from
// the inspection target. Unmounting (leaving the Pipelines tab) tears the
// transport down so no ACP connection leaks.
watch(
  () => selectedView.value,
  (view) => inspection.syncActiveNodes(view),
  { immediate: true }
);

// Re-attach when the FULL attachment identity changes: the selected pipeline,
// the target node, or any of the resolved session/agent/model fields. This
// mirrors the store's attachment key so a node id reused across pipelines, or a
// node whose session is upgraded (sessionRef → provenance.detail), forces a
// clean teardown+reconnect instead of bleeding the previous stream. `attach`
// is idempotent for an unchanged identity, so the 3s poll churn — which replaces
// the node object every tick with stable fields — does not thrash the transport.
watch(
  () => {
    const target = inspectionTarget.value;
    if (!target) return null;
    const guard = inspection.guardFor(target);
    return [
      store.selectedPipelineId ?? '',
      target.nodeId,
      guard.sessionId ?? '',
      guard.agent ?? '',
      guard.model ?? '',
    ].join('\u0000');
  },
  () => {
    void inspection.attach(inspectionTarget.value);
  },
  { immediate: true }
);

onUnmounted(() => {
  void inspection.detach();
});

interface Field {
  label: string;
  value: string;
  mono?: boolean;
}

function describe(node: ViewNode): Field[] {
  const fields: Field[] = [
    { label: 'id', value: node.nodeId, mono: true },
    { label: 'type', value: viewNodeType(node) },
    { label: 'status', value: node.status },
    { label: 'attempts', value: String(node.attempts) },
    { label: 'session', value: node.sessionId, mono: true },
  ];
  if (node.type === 'prompt') fields.push({ label: 'agent', value: node.agent });
  if (node.type === 'tool') fields.push({ label: 'tool', value: node.toolName, mono: true });
  if (node.type === 'sequentialThinking') fields.push({ label: 'topic', value: node.topic });
  const model = (node as { model?: string | null }).model;
  if (model) fields.push({ label: 'model', value: model, mono: true });
  return fields;
}

const fields = computed<Field[]>(() => (node.value ? describe(node.value) : []));

const promptPreview = computed(() => {
  const n = node.value;
  if (!n) return null;
  if (n.type === 'prompt') return n.prompt;
  // WorkflowViewNode.type is optional and defaults to 'workflow', so a workflow
  // node emitted without an explicit `type` must still match — use the guard.
  if (isWorkflowViewNode(n)) return n.prompt ?? null;
  return null;
});

const configEntries = computed<ConfigEntry[]>(() =>
  node.value ? nodeConfigEntries(node.value) : []
);

const outputPreview = computed(() => {
  const out = node.value?.output;
  if (!out) return null;
  return out.length > 280 ? `${out.slice(0, 280)}…` : out;
});

// Surfaces the zoom overlay can show for this node: its content surfaces plus
// the live ACP session (offered for any inspectable/non-tool node, even before
// it has produced output — that's exactly when the live stream matters).
const surfaceCount = computed(() => {
  const n = node.value;
  if (!n) return 0;
  return availableSurfaces(n).length + (isInspectableNode(n) ? 1 : 0);
});

function openZoom(): void {
  if (node.value) controller.openZoom(node.value.nodeId);
}

// --- inline node surgery (T-600.5) -----------------------------------------
// Resume (restart) an abandoned/stuck/failed node, or abandon an active node,
// directly on the LIVE pipeline. Enablement mirrors the verified backend guards
// (update_restart.rs / update_status.rs). Actions pause polling, mutate, re-view
// to prove persistence, then resume — all via the shared surgery controller.
const surgery = useSurgeryActions();
const restartState = computed(() => canRestartNode(selectedView.value, node.value));
const abandonState = computed(() => canAbandonNode(node.value));

function onRestart(): void {
  if (node.value) void surgery.restartNode(node.value.nodeId);
}

function onAbandon(): void {
  if (node.value) void surgery.abandonNode(node.value.nodeId);
}
</script>

<template>
  <Teleport defer to="#pipeline-inspector">
    <aside class="inspector" aria-label="Node inspector">
      <p v-if="!node" class="inspector-empty">
        <MousePointerClick :size="15" :stroke-width="2" />
        Select a node to inspect it.
      </p>

      <template v-else>
        <header class="inspector-head">
          <span class="inspector-label">{{ viewNodeDisplayLabel(node) }}</span>
          <button
            type="button"
            class="inspector-zoom"
            :disabled="surfaceCount === 0"
            :title="surfaceCount === 0 ? 'No surfaces to zoom' : 'Open semantic zoom'"
            aria-label="Open semantic zoom"
            @click="openZoom"
          >
            <Maximize2 :size="14" :stroke-width="2" />
          </button>
        </header>

        <dl class="inspector-fields">
          <div v-for="field in fields" :key="field.label" class="inspector-field">
            <dt>{{ field.label }}</dt>
            <dd :class="{ mono: field.mono }">{{ field.value }}</dd>
          </div>
        </dl>

        <ProvenanceBreadcrumbs :node="node" />

        <!-- Inline node surgery: resume (restart) or abandon, gated by the
             backend-verified enablement matrix. -->
        <section
          v-if="restartState.enabled || abandonState.enabled"
          class="inspector-surgery"
        >
          <button
            v-if="restartState.enabled"
            type="button"
            class="surgery-btn resume"
            :disabled="surgery.busy.value"
            @click="onRestart"
          >
            <RotateCcw :size="13" :stroke-width="2" />
            Resume from this node
          </button>
          <button
            v-if="abandonState.enabled"
            type="button"
            class="surgery-btn abandon"
            :disabled="surgery.busy.value"
            @click="onAbandon"
          >
            <Octagon :size="13" :stroke-width="2" />
            Abandon node
          </button>
        </section>

        <section class="inspector-section">
          <SessionInspectorPanel :node="node" />
        </section>

        <section v-if="promptPreview" class="inspector-section">
          <h4>Prompt</h4>
          <pre class="inspector-pre">{{ promptPreview }}</pre>
        </section>

        <section v-if="configEntries.length" class="inspector-section">
          <h4>Config</h4>
          <dl class="inspector-config">
            <div v-for="entry in configEntries" :key="entry.label" class="inspector-cfg">
              <dt>{{ entry.label }}</dt>
              <dd v-if="!entry.block" class="inspector-cfg-inline">{{ entry.value }}</dd>
              <pre v-else class="inspector-pre inspector-cfg-block">{{ entry.value }}</pre>
            </div>
          </dl>
        </section>

        <section v-if="outputPreview" class="inspector-section">
          <h4>Output preview</h4>
          <pre class="inspector-pre">{{ outputPreview }}</pre>
        </section>

        <button
          v-if="surfaceCount > 0"
          type="button"
          class="inspector-expand"
          @click="openZoom"
        >
          <Maximize2 :size="13" :stroke-width="2" />
          Expand ({{ surfaceCount }} surface{{ surfaceCount === 1 ? '' : 's' }})
        </button>
      </template>
    </aside>
  </Teleport>
</template>

<style scoped>
.inspector {
  display: flex;
  flex-direction: column;
  gap: 0.65rem;
  width: 280px;
  max-height: 100%;
  padding: 0.75rem;
  border: 1px solid var(--border-color);
  border-radius: 10px;
  background: color-mix(in srgb, var(--bg-main) 90%, var(--bg-sidebar));
  box-shadow: 0 2px 10px rgba(0, 0, 0, 0.14);
  overflow-y: auto;
}

.inspector-empty {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  margin: 0;
  color: var(--text-muted);
  font-size: 0.8rem;
}

.inspector-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
}

.inspector-label {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 0.88rem;
  font-weight: 700;
  color: var(--text-primary);
}

.inspector-zoom {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  padding: 0.3rem;
  border: 1px solid var(--border-color);
  border-radius: 7px;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  transition: background 0.12s ease, color 0.12s ease;
}

.inspector-zoom:hover:not(:disabled) {
  color: var(--text-accent);
  background: var(--bg-hover);
}

.inspector-zoom:disabled {
  cursor: default;
  opacity: 0.45;
}

.inspector-fields {
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
  margin: 0;
}

.inspector-field {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 0.6rem;
}

.inspector-field dt {
  flex-shrink: 0;
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.02em;
  color: var(--text-muted);
}

.inspector-field dd {
  margin: 0;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 0.78rem;
  color: var(--text-primary);
  text-align: right;
}

.inspector-field dd.mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 0.74rem;
}

.inspector-section {
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
}

.inspector-surgery {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.surgery-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.35rem;
  padding: 0.4rem 0.6rem;
  border: 1px solid var(--border-color);
  border-radius: 7px;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  font: inherit;
  font-size: 0.78rem;
  font-weight: 600;
  transition: background 0.12s ease, color 0.12s ease;
}

.surgery-btn.resume {
  color: var(--text-accent);
  border-color: color-mix(in srgb, var(--text-accent) 40%, var(--border-color));
}

.surgery-btn.resume:hover:not(:disabled) {
  background: color-mix(in srgb, var(--text-accent) 12%, transparent);
}

.surgery-btn.abandon:hover:not(:disabled) {
  color: var(--bg-danger, #d9534f);
  border-color: color-mix(in srgb, var(--bg-danger, #d9534f) 45%, var(--border-color));
  background: var(--bg-hover);
}

.surgery-btn:disabled {
  cursor: default;
  opacity: 0.5;
}

.inspector-section h4 {
  margin: 0;
  font-size: 0.7rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.03em;
  color: var(--text-accent);
}

.inspector-config {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  margin: 0;
}

.inspector-cfg {
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
}

.inspector-cfg dt {
  font-size: 0.68rem;
  text-transform: uppercase;
  letter-spacing: 0.02em;
  color: var(--text-muted);
}

.inspector-cfg-inline {
  margin: 0;
  font-size: 0.76rem;
  color: var(--text-primary);
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  word-break: break-word;
}

.inspector-cfg-block {
  max-height: 9rem;
}

.inspector-pre {
  margin: 0;
  max-height: 8rem;
  padding: 0.5rem 0.6rem;
  border: 1px solid var(--border-color);
  border-radius: 7px;
  background: color-mix(in srgb, var(--bg-main) 94%, var(--bg-sidebar));
  color: var(--text-secondary);
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 0.74rem;
  line-height: 1.45;
  white-space: pre-wrap;
  word-break: break-word;
  overflow-y: auto;
}

.inspector-expand {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.35rem;
  padding: 0.4rem 0.6rem;
  border: 1px solid var(--border-color);
  border-radius: 7px;
  background: color-mix(in srgb, var(--text-accent) 10%, transparent);
  color: var(--text-accent);
  cursor: pointer;
  font-size: 0.78rem;
  font-weight: 600;
  transition: background 0.12s ease;
}

.inspector-expand:hover {
  background: color-mix(in srgb, var(--text-accent) 18%, transparent);
}
</style>
