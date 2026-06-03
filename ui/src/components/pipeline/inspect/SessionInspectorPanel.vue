<script setup lang="ts">
// Live ACP session inspector panel (T-600.7).
//
// Read-only LIVE message stream for the inspected pipeline node's ACP session.
// Presentational only: the attach/replay/cleanup lifecycle is owned by the
// pipelineInspection store (driven by NodeInspectorPane); this component reads
// the store's reactive state and renders it. It mirrors the Agents-tab chat
// rendering of session/update messages — but OBSERVES only: there is no prompt
// input and no permission UI.
//
// It also surfaces the GLOBAL hidden-inference banner: any active non-tool node
// reasoning without an inspectable session past the 30s grace window is listed
// here so the operator always sees it, even when a different node is selected.
import { computed } from 'vue';
import { storeToRefs } from 'pinia';
import { Activity, CircleAlert, Eye, LoaderCircle } from '@lucide/vue';
import { usePipelineInspectionStore } from '../../../stores/pipelineInspection';
import { isInspectableNode } from '../../../lib/sessionResolve';
import { type ViewNode } from '../../../lib/pipeline-types';

const props = defineProps<{ node: ViewNode | null }>();

const inspection = usePipelineInspectionStore();
const { attachedNodeId, attachState, attachError, messages, hiddenErrors } =
  storeToRefs(inspection);

const isTool = computed(() => (props.node ? !isInspectableNode(props.node) : false));
const guard = computed(() => (props.node ? inspection.guardFor(props.node) : null));
const active = computed(() => (props.node ? inspection.isNodeActive(props.node.nodeId) : false));
const grace = computed(() =>
  props.node ? inspection.graceInfo(props.node.nodeId) : { pastGrace: true, remainingMs: 0 }
);

// Is the store's single live attachment pointed at THIS panel's node?
const attachedHere = computed(
  () => props.node != null && attachedNodeId.value === props.node.nodeId
);

const graceSeconds = computed(() => Math.ceil(grace.value.remainingMs / 1000));

// The phase this panel should render for its node.
type Phase = 'none' | 'tool' | 'connecting' | 'live' | 'error' | 'cold-start' | 'hidden' | 'inert';
const phase = computed<Phase>(() => {
  if (!props.node) return 'none';
  if (isTool.value) return 'tool';
  const g = guard.value;
  if (g && g.ok) {
    if (attachedHere.value) {
      if (attachState.value === 'error') return 'error';
      if (attachState.value === 'live') return 'live';
    }
    return 'connecting';
  }
  // Guard fails: distinguish cold-start (active, within grace) from a genuine
  // hidden-inference error (active, past grace) from an inert/finished node.
  if (active.value) {
    return grace.value.pastGrace ? 'hidden' : 'cold-start';
  }
  return 'inert';
});

const missingLabel = computed(() => guard.value?.missing.join(', ') ?? '');
</script>

<template>
  <section class="live" aria-label="Live ACP session">
    <header class="live-head">
      <Eye :size="14" :stroke-width="2" />
      <span class="live-title">Live session</span>
      <span v-if="phase === 'live'" class="live-badge live-badge--on">
        <Activity :size="12" :stroke-width="2.2" /> streaming
      </span>
      <span v-else-if="phase === 'connecting'" class="live-badge">
        <LoaderCircle :size="12" :stroke-width="2.2" class="spin" /> connecting
      </span>
    </header>

    <!-- GLOBAL hidden-inference banner: every active node reasoning without an
         inspectable session past grace, regardless of current selection. -->
    <div v-if="hiddenErrors.length" class="hidden-banner" role="alert">
      <p class="hidden-banner-title">
        <CircleAlert :size="14" :stroke-width="2.2" />
        Hidden inference — {{ hiddenErrors.length }} active node{{ hiddenErrors.length === 1 ? '' : 's' }} not inspectable
      </p>
      <ul class="hidden-list">
        <li v-for="err in hiddenErrors" :key="err.nodeId">
          <span class="hidden-node">{{ err.displayLabel }}</span>
          <span class="hidden-missing">missing {{ err.missing.join(', ') }}</span>
        </li>
      </ul>
    </div>

    <p v-if="phase === 'none'" class="live-empty">Select a node to observe its session.</p>
    <p v-else-if="phase === 'tool'" class="live-empty">Tool nodes have no ACP session to observe.</p>
    <p v-else-if="phase === 'inert'" class="live-empty">
      This node has no inspectable session (missing {{ missingLabel }}).
    </p>

    <p v-else-if="phase === 'cold-start'" class="live-cold" role="status">
      <LoaderCircle :size="14" :stroke-width="2.2" class="spin" />
      Cold-starting ACP session… {{ graceSeconds }}s
    </p>

    <div v-else-if="phase === 'hidden'" class="hidden-node-error" role="alert">
      <p class="hidden-banner-title">
        <CircleAlert :size="14" :stroke-width="2.2" />
        Hidden inference path
      </p>
      <p class="hidden-detail">
        This active node is reasoning without an inspectable session: missing
        <strong>{{ missingLabel }}</strong>. maenifold cannot show its live stream.
      </p>
    </div>

    <p v-else-if="phase === 'connecting'" class="live-cold" role="status">
      <LoaderCircle :size="14" :stroke-width="2.2" class="spin" />
      Connecting to session…
    </p>

    <p v-else-if="phase === 'error'" class="live-error" role="alert">
      <CircleAlert :size="14" :stroke-width="2.2" />
      {{ attachError ?? 'Session connection failed.' }}
    </p>

    <div v-else-if="phase === 'live'" class="stream">
      <p v-if="messages.length === 0" class="live-empty">
        Connected — waiting for the agent to stream output…
      </p>
      <article
        v-for="msg in messages"
        :key="msg.id"
        class="msg"
        :class="`msg--${msg.role}`"
      >
        <span class="msg-role">{{ msg.role }}</span>
        <p v-if="msg.thought" class="msg-thought">{{ msg.thought }}</p>
        <p v-if="msg.content" class="msg-content">{{ msg.content }}</p>
        <ul v-if="msg.toolCalls && msg.toolCalls.length" class="msg-tools">
          <li v-for="tc in msg.toolCalls" :key="tc.toolCallId" :class="`tool tool--${tc.status}`">
            <span class="tool-title">{{ tc.title }}</span>
            <span class="tool-status">{{ tc.status }}</span>
          </li>
        </ul>
      </article>
    </div>
  </section>
</template>

<style scoped>
.live {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
  min-height: 0;
}

.live-head {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  color: var(--text-accent);
}

.live-title {
  font-size: 0.7rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.03em;
}

.live-badge {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  margin-left: auto;
  padding: 0.05rem 0.4rem;
  border-radius: 10px;
  background: var(--bg-hover);
  color: var(--text-secondary);
  font-size: 0.66rem;
  text-transform: uppercase;
  letter-spacing: 0.02em;
}

.live-badge--on {
  background: color-mix(in srgb, var(--node-status-completed, #6cc070) 22%, transparent);
  color: color-mix(in srgb, var(--node-status-completed, #6cc070) 85%, var(--text-primary));
}

.live-empty,
.live-cold,
.live-error {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  margin: 0;
  font-size: 0.78rem;
  color: var(--text-muted);
}

.live-cold {
  color: var(--text-secondary);
}

.live-error {
  color: var(--text-primary);
}

.hidden-banner,
.hidden-node-error {
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
  padding: 0.5rem 0.55rem;
  border: 1px solid color-mix(in srgb, var(--bg-danger) 55%, var(--border-color));
  border-radius: 8px;
  background: color-mix(in srgb, var(--bg-danger) 14%, var(--bg-main));
}

.hidden-banner-title {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  margin: 0;
  font-size: 0.74rem;
  font-weight: 700;
  color: var(--text-primary);
}

.hidden-list {
  margin: 0;
  padding-left: 0.2rem;
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
}

.hidden-list li {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 0.5rem;
  font-size: 0.72rem;
}

.hidden-node {
  color: var(--text-primary);
  font-weight: 600;
}

.hidden-missing {
  color: var(--text-secondary);
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 0.68rem;
}

.hidden-detail {
  margin: 0;
  font-size: 0.74rem;
  color: var(--text-secondary);
  line-height: 1.4;
}

.stream {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
  min-height: 0;
  max-height: 22rem;
  overflow-y: auto;
  padding-right: 0.2rem;
}

.msg {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
  padding: 0.4rem 0.5rem;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: color-mix(in srgb, var(--bg-main) 95%, var(--bg-sidebar));
}

.msg--user {
  background: color-mix(in srgb, var(--node-accent-prompt, #adc6ff) 12%, var(--bg-main));
}

.msg-role {
  font-size: 0.62rem;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--text-muted);
}

.msg-thought {
  margin: 0;
  padding-left: 0.5rem;
  border-left: 2px solid var(--border-color);
  color: var(--text-muted);
  font-style: italic;
  font-size: 0.74rem;
  white-space: pre-wrap;
  word-break: break-word;
}

.msg-content {
  margin: 0;
  color: var(--text-primary);
  font-size: 0.78rem;
  line-height: 1.45;
  white-space: pre-wrap;
  word-break: break-word;
}

.msg-tools {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
  margin: 0.15rem 0 0;
  padding: 0;
  list-style: none;
}

.tool {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  padding: 0.2rem 0.4rem;
  border-radius: 6px;
  background: var(--bg-hover);
  font-size: 0.7rem;
}

.tool-title {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  color: var(--text-secondary);
}

.tool-status {
  flex-shrink: 0;
  text-transform: uppercase;
  font-size: 0.62rem;
  letter-spacing: 0.02em;
  color: var(--text-muted);
}

.tool--completed .tool-status {
  color: color-mix(in srgb, var(--node-status-completed, #6cc070) 85%, var(--text-primary));
}

.tool--failed .tool-status {
  color: var(--bg-danger);
}

.spin {
  animation: live-spin 0.9s linear infinite;
}

@keyframes live-spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
