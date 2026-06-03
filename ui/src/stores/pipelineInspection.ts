// Live ACP inspection store (T-600.7).
//
// Owns the read-only observation of a running pipeline node's ACP session for
// DISPLAY — the opposite of hidden inference. Two responsibilities:
//
//  1. GUARD/GRACE tracking for ALL active non-tool nodes: records when each
//     becomes active, and after a 30s cold-start grace window surfaces a
//     HIDDEN-INFERENCE error for any active node still missing sessionId/agent/
//     model. The operator must always see that maenifold is reasoning through an
//     inspectable session.
//  2. A single live ATTACHMENT for the currently-inspected node: it reuses the
//     session store's loadSession + onSessionUpdate pattern (stores/session.ts:
//     621-707) to reconnect to the node's existing ACP session by sessionId and
//     fold `session/update` notifications into a renderable buffer. Exactly one
//     transport is live at a time; switching nodes / unmounting disconnects it so
//     no transport leaks.
//
// This is a CLIENT observing the daemon's ACP session (the canonical product ACP
// path: createAcpClient → WebSocket /api/acp/session?agent=<name>). It does NOT
// route through the MCP gateway, send prompts, or drive permission UI.

import { computed, reactive, ref } from 'vue';
import { defineStore } from 'pinia';
import { AcpClientBridge, createAcpClient } from '../lib/acp-bridge';
import { applySessionUpdate, type ReplayState } from '../lib/acpReplay';
import {
  graceRemainingMs,
  hiddenInferenceErrors,
  inspectionGuard,
  isInspectableNode,
  isPastGrace,
  type InspectionGuard,
} from '../lib/sessionResolve';
import type { ChatMessage } from '../lib/types';
import { type PipelineView, type ViewNode } from '../lib/pipeline-types';
import { useConfigStore } from './config';
import { usePipelinesStore } from './pipelines';

// Matches the session store's connection envelope so the observer initialises the
// daemon ACP relay identically to the Agents tab.
const PROTOCOL_VERSION = 1;
const CLIENT_VERSION = '0.1.0';
// ACP loadSession requires a cwd string; a node may not carry one. The session is
// located by sessionId on the daemon regardless of cwd, so '.' is a protocol
// placeholder, not a behavioural default.
const FALLBACK_CWD = '.';

export type AttachState = 'idle' | 'connecting' | 'live' | 'blocked' | 'error';

const EMPTY_MESSAGES: ChatMessage[] = [];

function toMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function nodeCwd(node: ViewNode): string {
  const cwd = (node as { cwd?: string | null }).cwd;
  return typeof cwd === 'string' && cwd.trim().length > 0 ? cwd : FALLBACK_CWD;
}

export const usePipelineInspectionStore = defineStore('pipelineInspection', () => {
  const config = useConfigStore();
  const pipelines = usePipelinesStore();

  // --- grace / guard tracking ----------------------------------------------
  // graceStartByNode: nodeId → timestamp the node first became active.
  const graceStartByNode = ref(new Map<string, number>());
  // A 1s clock that ticks only while active nodes are tracked, so the
  // hidden-inference derivation re-evaluates as the grace window elapses.
  const nowTick = ref(Date.now());
  let tickTimer: ReturnType<typeof setInterval> | null = null;

  function stopTick(): void {
    if (tickTimer !== null) {
      clearInterval(tickTimer);
      tickTimer = null;
    }
  }

  function ensureTick(): void {
    if (tickTimer !== null) return;
    tickTimer = setInterval(() => {
      nowTick.value = Date.now();
      if (graceStartByNode.value.size === 0) stopTick();
    }, 1000);
  }

  // Reconcile the grace map against the live view's active set. Called on every
  // poll (selectedView is replaced each tick).
  function syncActiveNodes(view: PipelineView | null): void {
    const activeIds = new Set(view?.active ?? []);
    const nodesById = new Map((view?.nodes ?? []).map((n) => [n.nodeId, n]));
    const now = Date.now();
    for (const id of activeIds) {
      const node = nodesById.get(id);
      if (!node || !isInspectableNode(node)) continue;
      if (!graceStartByNode.value.has(id)) graceStartByNode.value.set(id, now);
    }
    for (const id of [...graceStartByNode.value.keys()]) {
      if (!activeIds.has(id)) graceStartByNode.value.delete(id);
    }
    nowTick.value = now;
    if (graceStartByNode.value.size > 0) ensureTick();
    else stopTick();
  }

  // The active non-tool nodes that are reasoning without an inspectable session
  // past the grace window. The operator MUST see these.
  const hiddenErrors = computed(() => {
    const view = pipelines.selectedView;
    if (!view) return [];
    const activeIds = new Set(view.active);
    return hiddenInferenceErrors(view.nodes, activeIds, graceStartByNode.value, nowTick.value);
  });

  function guardFor(node: ViewNode): InspectionGuard {
    return inspectionGuard(node);
  }

  // Grace status for a node: whether the window has elapsed and how long remains.
  function graceInfo(nodeId: string): { pastGrace: boolean; remainingMs: number } {
    const start = graceStartByNode.value.get(nodeId);
    return {
      pastGrace: isPastGrace(start, nowTick.value),
      remainingMs: graceRemainingMs(start, nowTick.value),
    };
  }

  function isNodeActive(nodeId: string): boolean {
    return (pipelines.selectedView?.active ?? []).includes(nodeId);
  }

  // --- single live attachment ----------------------------------------------
  // attachedNodeId is the DISPLAY identity (which node the panel is showing).
  // attachedKey is the FULL ATTACHMENT IDENTITY — pipelineId|nodeId|sessionId|
  // agent|model — so that a node id reused across two different pipelines, or a
  // node whose session is upgraded (sessionRef → provenance.detail), is treated
  // as a DIFFERENT attachment and never bleeds the previous node's stream.
  const attachedNodeId = ref<string | null>(null);
  const attachedKey = ref<string | null>(null);
  const attachState = ref<AttachState>('idle');
  const attachError = ref<string | null>(null);

  // Replay buffers keyed by the full attachment identity (not bare nodeId), so a
  // stale stream can never be served to a coincidentally-equal node id. Each
  // buffer's array is reactive so streamed pushes re-render the panel.
  const buffers = new Map<string, ReplayState>();
  // The live bridge. Held outside reactive state — exactly one at a time.
  let acpClient: AcpClientBridge | null = null;

  // The full attachment identity for a fully-resolved node. Includes the selected
  // pipeline id so the same node id under a different pipeline is a distinct key.
  function attachmentKey(
    node: ViewNode,
    sessionId: string,
    agent: string,
    model: string | null
  ): string {
    const pipelineId = pipelines.selectedPipelineId ?? '';
    return [pipelineId, node.nodeId, sessionId, agent, model ?? ''].join('\u0000');
  }

  const messages = computed<ChatMessage[]>(() => {
    const key = attachedKey.value;
    if (!key) return EMPTY_MESSAGES;
    return buffers.get(key)?.messages ?? EMPTY_MESSAGES;
  });

  function resetBuffer(key: string): ReplayState {
    const state: ReplayState = { messages: reactive<ChatMessage[]>([]), toolCalls: new Map() };
    buffers.set(key, state);
    return state;
  }

  async function teardownClient(client: AcpClientBridge | null): Promise<void> {
    if (!client) return;
    client.onSessionUpdate = null;
    client.onTransportClose = null;
    try {
      await client.disconnect();
    } catch (e) {
      console.warn('pipelineInspection: disconnect failed:', e);
    }
  }

  // Disconnect the live attachment and clear attach state. Idempotent.
  async function detach(): Promise<void> {
    attachState.value = 'idle';
    attachError.value = null;
    attachedNodeId.value = null;
    attachedKey.value = null;
    buffers.clear();
    const client = acpClient;
    acpClient = null;
    await teardownClient(client);
  }

  // Attach (or reconnect) the live observer to a node. No-op for tool nodes.
  // Reconnect is idempotent on the FULL attachment identity: an already-live/
  // connecting attachment with the same pipeline+node+session+agent+model is left
  // alone, so the poll-driven node-object churn doesn't thrash the transport — but
  // ANY identity change (blocked/errored retry, pipeline switch reusing a node id,
  // or a session upgrade) tears down and reconnects cleanly.
  async function attach(node: ViewNode | null): Promise<void> {
    if (!node || !isInspectableNode(node)) {
      await detach();
      return;
    }

    const guard = inspectionGuard(node);
    // The prospective identity is only meaningful once the node is fully resolved.
    const key =
      guard.ok && guard.sessionId && guard.agent
        ? attachmentKey(node, guard.sessionId, guard.agent, guard.model)
        : null;

    if (
      key !== null &&
      attachedKey.value === key &&
      (attachState.value === 'live' || attachState.value === 'connecting')
    ) {
      return;
    }

    await detach();
    attachedNodeId.value = node.nodeId;

    if (key === null || !guard.sessionId || !guard.agent) {
      // Reasoning without a fully-resolved inspectable session. The panel shows
      // the grace countdown or the hidden-inference error; we do not connect.
      attachState.value = 'blocked';
      return;
    }
    attachedKey.value = key;
    const sessionId = guard.sessionId;
    const agentName = guard.agent;

    const agentConfig = config.getAgent(agentName);
    if (!agentConfig) {
      attachState.value = 'error';
      attachError.value = `Agent '${agentName}' is not present in the agents config`;
      return;
    }

    attachState.value = 'connecting';
    attachError.value = null;
    const buffer = resetBuffer(key);

    let client: AcpClientBridge | null = null;
    try {
      // Read-only observer: force the bridge's fs handlers OFF regardless of
      // hasLocalFs(). Without this the bridge defaults fsAvailable to
      // hasLocalFs() (acp-bridge.ts:79) and would service fs/read_text_file and
      // fs/write_text_file RPCs from the observed session — breaking the
      // read-only guarantee. Advertising fs:false in clientCapabilities below is
      // not sufficient: it is advisory, the RPC handler gate is fsAvailable.
      client = await createAcpClient(
        { name: agentName, config: agentConfig },
        { fsAvailable: false }
      );
      if (attachedKey.value !== key) {
        await teardownClient(client);
        return;
      }
      acpClient = client;
      const activeClient = client;
      activeClient.onSessionUpdate = (notification) => {
        if (attachedKey.value === key) {
          applySessionUpdate(buffer, notification);
        }
      };
      activeClient.onTransportClose = (reason) => {
        if (attachedKey.value === key) {
          attachState.value = 'error';
          attachError.value = `Connection lost: ${reason ?? 'transport closed'}`;
        }
      };

      // Read-only observer: advertise NO filesystem capability (we are not the
      // host for this session) so a replay can never drive our local fs.
      await activeClient.initialize({
        protocolVersion: PROTOCOL_VERSION,
        clientCapabilities: { fs: { readTextFile: false, writeTextFile: false } },
        clientInfo: { name: 'maenifold-ui', title: 'Maenifold', version: CLIENT_VERSION },
      });
      if (attachedKey.value !== key) {
        await teardownClient(activeClient);
        if (acpClient === activeClient) acpClient = null;
        return;
      }

      // Reconnect to the node's existing ACP session. The agent replays the
      // conversation via session/update notifications, which onSessionUpdate
      // folds into the buffer — exactly the session store's resume pattern.
      await activeClient.loadSession({ sessionId, cwd: nodeCwd(node), mcpServers: [] });
      if (attachedKey.value !== key) {
        await teardownClient(activeClient);
        if (acpClient === activeClient) acpClient = null;
        return;
      }
      attachState.value = 'live';
    } catch (e) {
      if (client) await teardownClient(client);
      if (acpClient === client) acpClient = null;
      if (attachedKey.value === key) {
        attachState.value = 'error';
        attachError.value = toMessage(e);
      }
    }
  }

  return {
    // grace / guard
    graceStartByNode,
    nowTick,
    hiddenErrors,
    syncActiveNodes,
    guardFor,
    graceInfo,
    isNodeActive,
    // live attachment
    attachedNodeId,
    attachedKey,
    attachState,
    attachError,
    messages,
    attach,
    detach,
  };
});
