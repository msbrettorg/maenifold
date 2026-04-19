<script setup lang="ts">
import { computed, ref } from 'vue';
import AddNodeForm from '../../../AddNodeForm.vue';
import type { PipelineNodeDraftByType } from '../../../../lib/pipeline-node-factory';
import type { McpServerDiscoveryResult } from '../../../../lib/types';
import { useConfigStore } from '../../../../stores/config';
import { useMcpToolDiscoveryStore } from '../../../../stores/mcp-tool-discovery';
import { useNodePaletteStore } from '../../../../stores/node-palette';
import { usePipelineStore } from '../../../../stores/pipeline';

type EvidenceNodeType = 'workflow' | 'prompt' | 'sequentialThinking' | 'tool';

interface EvidenceNode {
  id: number;
  type: EvidenceNodeType;
  summary: string;
}

function cloneDraft<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

const workflows = ['workflow.alpha', 'workflow.beta'];
const capturedNodes = ref<EvidenceNode[]>([]);
const nodeCounter = ref(0);

const configStore = useConfigStore();
configStore.config = {
  mcpServers: [],
  agents: {
    architect: { command: 'architect' },
  },
};

const discoveryResults: McpServerDiscoveryResult[] = [
  {
    server: 'maenifold',
    type: 'stdio',
    status: 'ok',
    tools: [
      {
        name: 'maenifold.WriteMemory',
        description: 'Write memory notes to the graph safely.',
        inputSchema: {
          type: 'object',
          required: ['title'],
          properties: {
            title: { type: 'string', title: 'Title' },
          },
        },
      },
      {
        name: 'maenifold.SearchMemories',
        description: 'Search memory notes by query text.',
        inputSchema: {
          type: 'object',
          required: ['query'],
          properties: {
            query: { type: 'string', title: 'Query' },
          },
        },
      },
    ],
  },
  {
    server: 'memory-tools',
    type: 'stdio',
    status: 'ok',
    tools: [
      {
        name: 'memory-tools.MemoryDigest',
        description: 'Summarize memory matches into a compact digest.',
        inputSchema: {
          type: 'object',
          properties: {
            limit: { type: 'integer', title: 'Limit', minimum: 1 },
          },
        },
      },
      {
        name: 'memory-tools.MemoryNoteLookup',
        description: 'Look up a memory note by exact title.',
        inputSchema: {
          type: 'object',
          required: ['title'],
          properties: {
            title: { type: 'string', title: 'Title' },
          },
        },
      },
    ],
  },
  {
    server: 'filesystem',
    type: 'stdio',
    status: 'ok',
    tools: [
      {
        name: 'filesystem.ReadFile',
        description: 'Read a file from disk.',
        inputSchema: {
          type: 'object',
          required: ['path'],
          properties: {
            path: { type: 'string', title: 'Path' },
          },
        },
      },
    ],
  },
];

const discoveryStore = useMcpToolDiscoveryStore();
discoveryStore.serverResults = discoveryResults;
discoveryStore.hasLoadedCatalog = true;

const pipelineStore = usePipelineStore();
pipelineStore.workflows = workflows;
pipelineStore.tools = discoveryResults.flatMap((result) =>
  result.tools.map((tool) => ({
    name: tool.name,
    description: tool.description ?? undefined,
    inputSchema: tool.inputSchema,
  })),
);

const nodePaletteStore = useNodePaletteStore();
nodePaletteStore.recentTools = ['filesystem.ReadFile'];
nodePaletteStore.pinnedTools = ['maenifold.SearchMemories'];

const evidenceSummary = computed(() => ({
  recentTools: nodePaletteStore.recentTools,
  pinnedTools: nodePaletteStore.pinnedTools,
}));

function describeNode(type: EvidenceNodeType, fields: PipelineNodeDraftByType[EvidenceNodeType]): string {
  if (type !== 'tool') {
    return type;
  }

  const toolDraft = fields as PipelineNodeDraftByType['tool'];
  const args = Object.entries(toolDraft.toolArgs ?? {})
    .map(([key, value]) => `${key}=${String(value)}`)
    .join(', ');

  return `${toolDraft.toolName}${args ? ` (${args})` : ''}`;
}

function submitNode<T extends EvidenceNodeType>(type: T, fields: PipelineNodeDraftByType[T]): void {
  nodeCounter.value += 1;
  capturedNodes.value.unshift({
    id: nodeCounter.value,
    type,
    summary: describeNode(type, cloneDraft(fields)),
  });
}
</script>

<template>
  <main class="evidence-page">
    <header class="evidence-header">
      <div>
        <p class="evidence-kicker">T-125.3 remediation loop 2</p>
        <h1>Node palette UX evidence harness</h1>
        <p>
          This page mounts the shipped <code>AddNodeForm</code> + <code>ToolNodeFields</code>
          components with a fixed MCP catalog so the red-team acceptance screenshots are reproducible.
        </p>
      </div>
      <dl class="evidence-meta">
        <div>
          <dt>Recent</dt>
          <dd>{{ evidenceSummary.recentTools.join(', ') }}</dd>
        </div>
        <div>
          <dt>Pinned</dt>
          <dd>{{ evidenceSummary.pinnedTools.join(', ') }}</dd>
        </div>
      </dl>
    </header>

    <section class="evidence-layout">
      <AddNodeForm
        :workflows="workflows"
        :has-selection="capturedNodes.length > 0"
        :submit-node="submitNode"
      />

      <aside class="evidence-sidebar" data-testid="captured-nodes">
        <div class="evidence-sidebar__head">
          <h2>Captured adds</h2>
          <span class="evidence-badge">{{ capturedNodes.length }}</span>
        </div>
        <p class="evidence-sidebar__copy">
          Successful submissions appear here so screenshots can show that the palette stayed open after add.
        </p>
        <ol v-if="capturedNodes.length" class="evidence-list">
          <li v-for="node in capturedNodes" :key="node.id">
            <strong>{{ node.type }}</strong>
            <span>{{ node.summary }}</span>
          </li>
        </ol>
        <p v-else class="evidence-empty">No nodes added yet.</p>
      </aside>
    </section>
  </main>
</template>

<style scoped>
.evidence-page {
  min-height: 100vh;
  padding: 2rem;
  background:
    radial-gradient(circle at top, color-mix(in srgb, var(--color-accent-primary) 16%, transparent), transparent 42%),
    linear-gradient(180deg, color-mix(in srgb, var(--bg-panel) 92%, #02030a), #05060d);
  color: var(--text-primary);
}

.evidence-header,
.evidence-layout,
.evidence-sidebar,
.evidence-list li {
  display: grid;
  gap: 1rem;
}

.evidence-header {
  grid-template-columns: minmax(0, 1fr) minmax(16rem, 24rem);
  align-items: start;
  margin: 0 auto 1.5rem;
  max-width: 1440px;
}

.evidence-kicker,
.evidence-sidebar__copy,
.evidence-empty,
.evidence-meta dt,
.evidence-meta dd,
code {
  color: var(--text-secondary);
}

.evidence-kicker {
  margin-bottom: 0.35rem;
  font-size: var(--text-xs);
  font-weight: 700;
  letter-spacing: 0.16em;
}

.evidence-header h1 {
  margin-bottom: 0.5rem;
  font-size: clamp(1.8rem, 3vw, 2.6rem);
}

.evidence-header p {
  max-width: 72ch;
}

.evidence-meta,
.evidence-meta div {
  display: grid;
  gap: 0.35rem;
}

.evidence-meta {
  padding: 1rem 1.1rem;
  border: 1px solid var(--border-default);
  border-radius: 1rem;
  background: color-mix(in srgb, var(--bg-main) 82%, transparent);
}

.evidence-meta dt {
  font-size: var(--text-xs);
  font-weight: 700;
  letter-spacing: 0.08em;
}

.evidence-meta dd {
  margin: 0;
  font-family: var(--font-mono);
  font-size: var(--text-sm);
}

.evidence-layout {
  grid-template-columns: minmax(820px, 1fr) minmax(18rem, 22rem);
  align-items: start;
  margin: 0 auto;
  max-width: 1440px;
}

.evidence-sidebar {
  position: sticky;
  top: 2rem;
  padding: 1rem 1.1rem;
  border: 1px solid var(--border-default);
  border-radius: 1rem;
  background: color-mix(in srgb, var(--bg-main) 84%, transparent);
}

.evidence-sidebar__head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 0.75rem;
}

.evidence-sidebar__head h2 {
  margin: 0;
  font-size: 1rem;
}

.evidence-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 1.75rem;
  min-height: 1.75rem;
  padding: 0 0.55rem;
  border-radius: 4px;
  background: color-mix(in srgb, var(--color-accent-primary) 18%, transparent);
  color: var(--color-accent-primary);
  font-weight: 700;
}

.evidence-list {
  margin: 0;
  padding-left: 1.1rem;
}

.evidence-list li {
  gap: 0.25rem;
}

.evidence-list span {
  font-family: var(--font-mono);
  font-size: var(--text-sm);
}

.evidence-empty {
  margin: 0;
}

@media (max-width: 1200px) {
  .evidence-header,
  .evidence-layout {
    grid-template-columns: 1fr;
  }

  .evidence-sidebar {
    position: static;
  }
}
