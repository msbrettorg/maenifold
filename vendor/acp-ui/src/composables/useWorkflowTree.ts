import { ref, type Ref } from 'vue';
import { useMemoryApi } from './useMemoryApi';
import type {
  MemoryFile,
  ThinkingSession,
  Thought,
  WorkflowSession,
  WorkflowStep,
  WorkflowTreeNode,
} from '../lib/workflow-types';

const STEP_HEADING_RE = /^Step (\d+)\/(\d+)$/;
const STEP_RESPONSE_HEADING_RE = /^Step (\d+)\/(\d+) Response$/;
const THOUGHT_HEADING_RE = /^Thought (\d+)\/(\d+) \[([^\]]+)\]/;
const STARTED_RE = /^Started:\s*(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2} UTC)$/m;
const SESSION_ID_RE = /\bsession-\d+-\d+\b/g;
const WORKFLOW_ID_RE = /\bworkflow-\d+\b/g;
const TIMESTAMP_LINE_RE = /^\*(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2} UTC)\*$/;
const H2_SECTION_RE = /^##\s+(.+)$/gm;

type MemoryApi = ReturnType<typeof useMemoryApi>;
type WorkflowStatus = WorkflowSession['status'];
type WorkflowStepStatus = WorkflowStep['status'];

type SessionLocatorKind = 'workflow' | 'sequential';

interface DateParts {
  year: string;
  month: string;
  day: string;
}

interface H2Section {
  heading: string;
  body: string;
}

interface WorkflowReferenceMap {
  childWorkflowIdsByStep: Map<number, string[]>;
}

function normalizeMarkdownBlock(value: string): string {
  return value.trim();
}

function toIsoUtc(value: string | null | undefined): string | null {
  if (!value) {
    return null;
  }

  const trimmed = value.trim();
  const utcMatch = trimmed.match(/^(\d{4}-\d{2}-\d{2}) (\d{2}:\d{2}:\d{2}) UTC$/);
  if (utcMatch) {
    return `${utcMatch[1]}T${utcMatch[2]}Z`;
  }

  const timestamp = Date.parse(trimmed);
  if (Number.isNaN(timestamp)) {
    return null;
  }

  return new Date(timestamp).toISOString();
}

function extractDateParts(created: string | null | undefined, uri: string): DateParts | null {
  const createdIso = toIsoUtc(created);
  if (createdIso) {
    const [year, month, day] = createdIso.slice(0, 10).split('-');
    return { year, month, day };
  }

  const uriMatch = uri.match(/^memory:\/\/thinking\/(?:workflow|sequential)\/(\d{4})\/(\d{2})\/(\d{2})\//);
  if (!uriMatch) {
    return null;
  }

  return {
    year: uriMatch[1],
    month: uriMatch[2],
    day: uriMatch[3],
  };
}

function buildMemoryUri(kind: SessionLocatorKind, id: string, dateParts: DateParts): string {
  return `memory://thinking/${kind}/${dateParts.year}/${dateParts.month}/${dateParts.day}/${id}`;
}

function uniqueMatches(pattern: RegExp, value: string, exclude: Set<string> = new Set()): string[] {
  const matches = Array.from(value.matchAll(pattern), (match) => match[0]);
  return Array.from(new Set(matches)).filter((match) => !exclude.has(match));
}

function parseH2Sections(content: string): H2Section[] {
  const matches = Array.from(content.matchAll(H2_SECTION_RE));
  const sections: H2Section[] = [];

  for (let index = 0; index < matches.length; index += 1) {
    const match = matches[index];
    const heading = match[1].trim();
    const start = match.index! + match[0].length;
    const end = index + 1 < matches.length ? matches[index + 1].index! : content.length;
    const body = content.slice(start, end).trim();
    sections.push({ heading, body });
  }

  return sections;
}

function stripTimestampLines(body: string): { content: string; timestamp?: string } {
  const lines = body.split('\n');
  let timestamp: string | undefined;
  const keptLines: string[] = [];

  for (const line of lines) {
    const match = line.trim().match(TIMESTAMP_LINE_RE);
    if (match) {
      timestamp = match[1];
      continue;
    }
    keptLines.push(line);
  }

  return {
    content: keptLines.join('\n').trim(),
    timestamp: toIsoUtc(timestamp) ?? undefined,
  };
}

function parseWorkflowStepDefinition(body: string, stepNumber: number, totalSteps: number): {
  id: string;
  name: string;
  requiresSequentialThinking: boolean;
} {
  const trimmed = normalizeMarkdownBlock(body);
  const fallback = {
    id: `step-${stepNumber}`,
    name: `Step ${stepNumber}`,
    requiresSequentialThinking: false,
  };

  if (!trimmed) {
    return fallback;
  }

  try {
    const parsed = JSON.parse(trimmed) as Record<string, unknown>;
    const id = typeof parsed.id === 'string' && parsed.id.trim()
      ? parsed.id.trim()
      : fallback.id;
    const name = typeof parsed.name === 'string' && parsed.name.trim()
      ? parsed.name.trim()
      : `Step ${stepNumber}/${totalSteps}`;
    const sequentialFlag = parsed.requiresSequentialThinking ?? parsed.sequentialThinkingEnabled;

    return {
      id,
      name,
      requiresSequentialThinking: sequentialFlag === true,
    };
  } catch {
    const id = trimmed.match(/"id"\s*:\s*"([^"]+)"/)?.[1] ?? fallback.id;
    const name = trimmed.match(/"name"\s*:\s*"([^"]+)"/)?.[1] ?? trimmed.split('\n')[0]?.trim() ?? fallback.name;
    const sequentialFlag = /"(?:requiresSequentialThinking|sequentialThinkingEnabled)"\s*:\s*true/.test(trimmed);

    return {
      id,
      name,
      requiresSequentialThinking: sequentialFlag,
    };
  }
}

function deriveWorkflowIdentity(file: MemoryFile, sessionId: string, content: string): {
  workflowId: string;
  workflowName: string;
} {
  const title = file.title.trim();
  if (title && !/^Workflow Session\b/i.test(title) && title !== sessionId) {
    return { workflowId: title, workflowName: title };
  }

  const explicitId = content.match(/(?:^|\n)(?:Workflow(?: ID| Name)?|workflowId)\s*[:=]\s*`?([A-Za-z0-9_-]+(?:-[A-Za-z0-9_-]+)*)`?/i)?.[1];
  if (explicitId) {
    return { workflowId: explicitId, workflowName: explicitId };
  }

  return { workflowId: sessionId, workflowName: sessionId };
}

function deriveWorkflowStarted(content: string, file: MemoryFile): string {
  const startedMatch = content.match(STARTED_RE)?.[1];
  return toIsoUtc(startedMatch) ?? toIsoUtc(file.created) ?? toIsoUtc(file.modified) ?? new Date(0).toISOString();
}

function parseWorkflowSession(file: MemoryFile): {
  session: WorkflowSession;
  references: WorkflowReferenceMap;
} {
  const sections = parseH2Sections(file.content);
  const sessionId = file.uri.match(/(workflow-\d+)$/)?.[1] ?? file.title;
  const status: WorkflowStatus = /(^|\n)## Session Abandoned\b/m.test(file.content)
    ? 'abandoned'
    : /(^|\n)## Conclusion\b/m.test(file.content)
      ? 'completed'
      : 'active';

  const definitionBodies = new Map<number, { totalSteps: number; body: string }>();
  const responseBodies = new Map<number, { totalSteps: number; body: string; respondedAt?: string }>();
  const childWorkflowIdsByStep = new Map<number, string[]>();

  for (const section of sections) {
    const stepMatch = section.heading.match(STEP_HEADING_RE);
    if (stepMatch) {
      definitionBodies.set(Number(stepMatch[1]), {
        totalSteps: Number(stepMatch[2]),
        body: section.body,
      });
      continue;
    }

    const responseMatch = section.heading.match(STEP_RESPONSE_HEADING_RE);
    if (responseMatch) {
      const stepNumber = Number(responseMatch[1]);
      const cleaned = stripTimestampLines(section.body);
      responseBodies.set(stepNumber, {
        totalSteps: Number(responseMatch[2]),
        body: cleaned.content,
        respondedAt: cleaned.timestamp,
      });
      childWorkflowIdsByStep.set(
        stepNumber,
        uniqueMatches(WORKFLOW_ID_RE, cleaned.content, new Set([sessionId]))
      );
    }
  }

  const stepNumbers = Array.from(new Set([
    ...definitionBodies.keys(),
    ...responseBodies.keys(),
  ])).sort((left, right) => left - right);

  const totalSteps = stepNumbers.reduce((max, stepNumber) => {
    return Math.max(
      max,
      definitionBodies.get(stepNumber)?.totalSteps ?? 0,
      responseBodies.get(stepNumber)?.totalSteps ?? 0,
    );
  }, 0);

  let activeAssigned = false;
  const steps: WorkflowStep[] = stepNumbers.map((stepNumber) => {
    const definition = definitionBodies.get(stepNumber);
    const response = responseBodies.get(stepNumber);
    const stepDetails = parseWorkflowStepDefinition(definition?.body ?? '', stepNumber, totalSteps || 1);
    const submachineSessionId = response
      ? uniqueMatches(SESSION_ID_RE, response.body)[0]
      : undefined;

    let stepStatus: WorkflowStepStatus = 'pending';
    if (response?.body) {
      stepStatus = 'completed';
    } else if (!activeAssigned && status === 'active') {
      stepStatus = 'active';
      activeAssigned = true;
    }

    return {
      stepNumber,
      totalSteps: definition?.totalSteps ?? response?.totalSteps ?? totalSteps,
      id: stepDetails.id,
      name: stepDetails.name,
      status: stepStatus,
      response: response?.body || undefined,
      respondedAt: response?.respondedAt,
      submachineSessionId,
      requiresSequentialThinking: stepDetails.requiresSequentialThinking || Boolean(submachineSessionId),
    };
  });

  const identity = deriveWorkflowIdentity(file, sessionId, file.content);

  return {
    session: {
      sessionId,
      uri: file.uri,
      workflowId: identity.workflowId,
      workflowName: identity.workflowName,
      started: deriveWorkflowStarted(file.content, file),
      status,
      steps,
    },
    references: {
      childWorkflowIdsByStep,
    },
  };
}

function parseThinkingSession(file: MemoryFile): ThinkingSession {
  const sections = parseH2Sections(file.content);
  const sessionId = file.uri.match(/(session-\d+-\d+)$/)?.[1] ?? file.title;
  const status: ThinkingSession['status'] = /(^|\n)## Session Abandoned\b/m.test(file.content)
    ? 'abandoned'
    : /(^|\n)## Conclusion\b/m.test(file.content)
      ? 'completed'
      : 'active';

  const thoughts: Thought[] = [];
  let conclusion: string | undefined;

  for (const section of sections) {
    const thoughtMatch = section.heading.match(THOUGHT_HEADING_RE);
    if (thoughtMatch) {
      const cleaned = stripTimestampLines(section.body);
      thoughts.push({
        number: Number(thoughtMatch[1]),
        totalThoughts: Number(thoughtMatch[2]),
        agent: thoughtMatch[3].trim(),
        content: cleaned.content,
        timestamp: cleaned.timestamp ?? toIsoUtc(file.modified) ?? toIsoUtc(file.created) ?? new Date(0).toISOString(),
      });
      continue;
    }

    if (section.heading === 'Conclusion') {
      conclusion = normalizeMarkdownBlock(section.body) || undefined;
    }
  }

  return {
    sessionId,
    uri: file.uri,
    thoughts,
    conclusion,
    status,
  };
}

async function resolveMemoryFile(
  api: MemoryApi,
  kind: SessionLocatorKind,
  id: string,
  parent?: MemoryFile,
): Promise<MemoryFile> {
  const candidateUris = new Set<string>();
  const parentDateParts = parent ? extractDateParts(parent.created, parent.uri) : null;

  if (parentDateParts) {
    candidateUris.add(buildMemoryUri(kind, id, parentDateParts));
  }

  for (const candidateUri of candidateUris) {
    try {
      return await api.readMemory(candidateUri);
    } catch {
      // Fall through to search-based recovery below.
    }
  }

  const folder = `thinking/${kind}`;
  const search = await api.searchMemories(id, {
    folder,
    pageSize: 25,
  });

  const searchMatch = search.results.find((item) => item.uri.endsWith(`/${id}`) || item.uri.includes(`/${id}`));
  if (searchMatch) {
    return api.readMemory(searchMatch.uri);
  }

  const recent = await api.recentActivity({
    filter: 'thinking',
    limit: 200,
  });
  const recentMatch = recent.items.find((item) => item.uri.endsWith(`/${id}`) || item.uri.includes(`/${id}`));
  if (recentMatch) {
    return api.readMemory(recentMatch.uri);
  }

  throw new Error(`Unable to resolve ${kind} session ${id}`);
}

async function buildWorkflowNode(
  api: MemoryApi,
  file: MemoryFile,
  workflowCache: Map<string, Promise<WorkflowTreeNode>>,
  thinkingCache: Map<string, Promise<WorkflowTreeNode>>,
): Promise<WorkflowTreeNode> {
  const cached = workflowCache.get(file.uri);
  if (cached) {
    return cached;
  }

  const promise = (async () => {
    const parsed = parseWorkflowSession(file);
    const children: WorkflowTreeNode[] = [];
    let firstChildWorkflowUri: string | undefined;

    for (const step of parsed.session.steps) {
      if (step.submachineSessionId) {
        const thinkingFile = await resolveMemoryFile(api, 'sequential', step.submachineSessionId, file);
        children.push(await buildThinkingNode(thinkingFile, thinkingCache));
      }

      const childWorkflowIds = parsed.references.childWorkflowIdsByStep.get(step.stepNumber) ?? [];
      for (const childWorkflowId of childWorkflowIds) {
        const childWorkflowFile = await resolveMemoryFile(api, 'workflow', childWorkflowId, file);
        const childWorkflowNode = await buildWorkflowNode(api, childWorkflowFile, workflowCache, thinkingCache);
        if (childWorkflowNode.type === 'workflow' && !firstChildWorkflowUri) {
          firstChildWorkflowUri = childWorkflowNode.data.uri;
        }
        children.push(childWorkflowNode);
      }
    }

    if (firstChildWorkflowUri) {
      parsed.session.childWorkflowUri = firstChildWorkflowUri;
    }

    return {
      type: 'workflow',
      data: parsed.session,
      children,
    } satisfies WorkflowTreeNode;
  })();

  workflowCache.set(file.uri, promise);

  try {
    return await promise;
  } catch (error) {
    workflowCache.delete(file.uri);
    throw error;
  }
}

async function buildThinkingNode(
  file: MemoryFile,
  thinkingCache: Map<string, Promise<WorkflowTreeNode>>,
): Promise<WorkflowTreeNode> {
  const cached = thinkingCache.get(file.uri);
  if (cached) {
    return cached;
  }

  const promise = Promise.resolve({
    type: 'thinking',
    data: parseThinkingSession(file),
    children: [],
  } satisfies WorkflowTreeNode);

  thinkingCache.set(file.uri, promise);

  try {
    return await promise;
  } catch (error) {
    thinkingCache.delete(file.uri);
    throw error;
  }
}

export function useWorkflowTree(): {
  loadTree: (uri: string) => Promise<WorkflowTreeNode>;
  tree: Ref<WorkflowTreeNode | null>;
  loading: Ref<boolean>;
  error: Ref<string | null>;
} {
  const api = useMemoryApi();
  const tree = ref<WorkflowTreeNode | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);

  async function loadTree(uri: string): Promise<WorkflowTreeNode> {
    loading.value = true;
    error.value = null;

    const workflowCache = new Map<string, Promise<WorkflowTreeNode>>();
    const thinkingCache = new Map<string, Promise<WorkflowTreeNode>>();

    try {
      const rootFile = await api.readMemory(uri);
      const rootNode = await buildWorkflowNode(api, rootFile, workflowCache, thinkingCache);
      tree.value = rootNode;
      return rootNode;
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      error.value = message;
      throw err;
    } finally {
      loading.value = false;
    }
  }

  return {
    loadTree,
    tree,
    loading,
    error,
  };
}
