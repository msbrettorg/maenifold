import type {
  MemoryFile,
  RecentResult,
  SearchResult,
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

type SessionLocatorKind = 'workflow' | 'sequential';
type WorkflowStatus = WorkflowSession['status'];
type WorkflowStepStatus = WorkflowStep['status'];
interface DateParts { year: string; month: string; day: string; }
interface WorkflowReferences { childWorkflowIdsByStep: Map<number, string[]>; }
interface WorkflowTreeMemoryApi {
  readMemory(uri: string): Promise<MemoryFile>;
  searchMemories(query: string, opts?: { folder?: string; pageSize?: number }): Promise<SearchResult>;
  recentActivity(opts?: { filter?: 'thinking'; limit?: number }): Promise<RecentResult>;
}
function toIsoUtc(value: string | null | undefined): string | null {
  if (!value?.trim()) {
    return null;
  }
  const trimmed = value.trim();
  const utcMatch = trimmed.match(/^(\d{4}-\d{2}-\d{2}) (\d{2}:\d{2}:\d{2}) UTC$/);
  if (utcMatch) {
    return `${utcMatch[1]}T${utcMatch[2]}Z`;
  }
  const timestamp = Date.parse(trimmed);
  return Number.isNaN(timestamp) ? null : new Date(timestamp).toISOString();
}

function extractDateParts(created: string | null | undefined, uri: string): DateParts | null {
  const createdIso = toIsoUtc(created);
  if (createdIso) {
    const [year, month, day] = createdIso.slice(0, 10).split('-');
    return { year, month, day };
  }
  const uriMatch = uri.match(/^memory:\/\/thinking\/(?:workflow|sequential)\/(\d{4})\/(\d{2})\/(\d{2})\//);
  return uriMatch ? { year: uriMatch[1], month: uriMatch[2], day: uriMatch[3] } : null;
}

const buildMemoryUri = (kind: SessionLocatorKind, id: string, dateParts: DateParts): string =>
  `memory://thinking/${kind}/${dateParts.year}/${dateParts.month}/${dateParts.day}/${id}`;

function uniqueMatches(pattern: RegExp, value: string, exclude: Set<string> = new Set()): string[] {
  return [...new Set([...value.matchAll(pattern)].map((match) => match[0]))].filter((match) => !exclude.has(match));
}

function parseH2Sections(content: string): Array<{ heading: string; body: string }> {
  const matches = [...content.matchAll(H2_SECTION_RE)];
  return matches.map((match, index) => ({
    heading: match[1].trim(),
    body: content.slice(match.index! + match[0].length, matches[index + 1]?.index ?? content.length).trim(),
  }));
}

function stripTimestampLines(body: string): { content: string; timestamp?: string } {
  let timestamp: string | undefined;
  const content = body
    .split('\n')
    .filter((line) => {
      const match = line.trim().match(TIMESTAMP_LINE_RE);
      if (!match) {
        return true;
      }
      timestamp = match[1];
      return false;
    })
    .join('\n')
    .trim();
  return { content, timestamp: toIsoUtc(timestamp) ?? undefined };
}

function parseStatus(content: string): WorkflowStatus | ThinkingSession['status'] {
  if (/(^|\n)## Session Abandoned\b/m.test(content)) {
    return 'abandoned';
  }
  return /(^|\n)## Conclusion\b/m.test(content) ? 'completed' : 'active';
}

function parseWorkflowStepDefinition(body: string, stepNumber: number, totalSteps: number): Pick<WorkflowStep, 'id' | 'name' | 'requiresSequentialThinking'> {
  const trimmed = body.trim();
  const fallback = { id: `step-${stepNumber}`, name: `Step ${stepNumber}`, requiresSequentialThinking: false };
  if (!trimmed) {
    return fallback;
  }
  try {
    const parsed = JSON.parse(trimmed) as Record<string, unknown>;
    const sequentialFlag = parsed.requiresSequentialThinking ?? parsed.sequentialThinkingEnabled;
    return {
      id: typeof parsed.id === 'string' && parsed.id.trim() ? parsed.id.trim() : fallback.id,
      name: typeof parsed.name === 'string' && parsed.name.trim() ? parsed.name.trim() : `Step ${stepNumber}/${totalSteps}`,
      requiresSequentialThinking: sequentialFlag === true,
    };
  } catch {
    return {
      id: trimmed.match(/"id"\s*:\s*"([^"]+)"/)?.[1] ?? fallback.id,
      name: trimmed.match(/"name"\s*:\s*"([^"]+)"/)?.[1] ?? trimmed.split('\n')[0]?.trim() ?? fallback.name,
      requiresSequentialThinking: /"(?:requiresSequentialThinking|sequentialThinkingEnabled)"\s*:\s*true/.test(trimmed),
    };
  }
}

function deriveWorkflowIdentity(file: MemoryFile, sessionId: string): Pick<WorkflowSession, 'workflowId' | 'workflowName'> {
  const title = file.title.trim();
  if (title && !/^Workflow Session\b/i.test(title) && title !== sessionId) {
    return { workflowId: title, workflowName: title };
  }
  const explicitId = file.content.match(
    /(?:^|\n)(?:Workflow(?: ID| Name)?|workflowId)\s*[:=]\s*`?([A-Za-z0-9_-]+(?:-[A-Za-z0-9_-]+)*)`?/i
  )?.[1];
  return explicitId ? { workflowId: explicitId, workflowName: explicitId } : { workflowId: sessionId, workflowName: sessionId };
}

function parseWorkflowSession(file: MemoryFile): { session: WorkflowSession; references: WorkflowReferences } {
  const sections = parseH2Sections(file.content);
  const sessionId = file.uri.match(/(workflow-\d+)$/)?.[1] ?? file.title;
  const definitionBodies = new Map<number, { totalSteps: number; body: string }>();
  const responseBodies = new Map<number, { totalSteps: number; body: string; respondedAt?: string }>();
  const childWorkflowIdsByStep = new Map<number, string[]>();

  for (const section of sections) {
    const stepMatch = section.heading.match(STEP_HEADING_RE);
    if (stepMatch) {
      definitionBodies.set(Number(stepMatch[1]), { totalSteps: Number(stepMatch[2]), body: section.body });
      continue;
    }
    const responseMatch = section.heading.match(STEP_RESPONSE_HEADING_RE);
    if (!responseMatch) {
      continue;
    }
    const stepNumber = Number(responseMatch[1]);
    const cleaned = stripTimestampLines(section.body);
    responseBodies.set(stepNumber, { totalSteps: Number(responseMatch[2]), body: cleaned.content, respondedAt: cleaned.timestamp });
    childWorkflowIdsByStep.set(stepNumber, uniqueMatches(WORKFLOW_ID_RE, cleaned.content, new Set([sessionId])));
  }

  const stepNumbers = [...new Set([...definitionBodies.keys(), ...responseBodies.keys()])].sort((a, b) => a - b);
  const totalSteps = stepNumbers.reduce((max, stepNumber) => Math.max(max, definitionBodies.get(stepNumber)?.totalSteps ?? 0, responseBodies.get(stepNumber)?.totalSteps ?? 0), 0);
  const status = parseStatus(file.content) as WorkflowStatus;
  let activeAssigned = false;

  const steps = stepNumbers.map((stepNumber) => {
    const definition = definitionBodies.get(stepNumber);
    const response = responseBodies.get(stepNumber);
    const details = parseWorkflowStepDefinition(definition?.body ?? '', stepNumber, totalSteps || 1);
    const { requiresSequentialThinking, ...stepIdentity } = details;
    const submachineSessionId = response ? uniqueMatches(SESSION_ID_RE, response.body)[0] : undefined;
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
      response: response?.body || undefined,
      respondedAt: response?.respondedAt,
      submachineSessionId,
      status: stepStatus,
      ...stepIdentity,
      requiresSequentialThinking: requiresSequentialThinking || Boolean(submachineSessionId),
    } satisfies WorkflowStep;
  });

  const identity = deriveWorkflowIdentity(file, sessionId);
  return {
    session: {
      sessionId,
      uri: file.uri,
      started: toIsoUtc(file.content.match(STARTED_RE)?.[1]) ?? toIsoUtc(file.created) ?? toIsoUtc(file.modified) ?? new Date(0).toISOString(),
      status,
      steps,
      ...identity,
    },
    references: { childWorkflowIdsByStep },
  };
}

function parseThinkingSession(file: MemoryFile): ThinkingSession {
  const thoughts: Thought[] = [];
  let conclusion: string | undefined;
  for (const section of parseH2Sections(file.content)) {
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
      conclusion = section.body.trim() || undefined;
    }
  }
  return {
    sessionId: file.uri.match(/(session-\d+-\d+)$/)?.[1] ?? file.title,
    uri: file.uri,
    thoughts,
    conclusion,
    status: parseStatus(file.content) as ThinkingSession['status'],
  };
}

async function resolveMemoryFile(api: WorkflowTreeMemoryApi, kind: SessionLocatorKind, id: string, parent?: MemoryFile): Promise<MemoryFile> {
  const parentDateParts = parent ? extractDateParts(parent.created, parent.uri) : null;
  const candidateUri = parentDateParts ? buildMemoryUri(kind, id, parentDateParts) : null;
  if (candidateUri) {
    try {
      return await api.readMemory(candidateUri);
    } catch {
      // Fall through to search-based recovery.
    }
  }
  const search = await api.searchMemories(id, { folder: `thinking/${kind}`, pageSize: 25 });
  const searchMatch = search.results.find((item) => item.uri.endsWith(`/${id}`) || item.uri.includes(`/${id}`));
  if (searchMatch) {
    return api.readMemory(searchMatch.uri);
  }
  const recent = await api.recentActivity({ filter: 'thinking', limit: 200 });
  const recentMatch = recent.items.find((item) => item.uri.endsWith(`/${id}`) || item.uri.includes(`/${id}`));
  if (recentMatch) {
    return api.readMemory(recentMatch.uri);
  }
  throw new Error(`Unable to resolve ${kind} session ${id}`);
}

async function buildThinkingNode(file: MemoryFile, cache: Map<string, Promise<WorkflowTreeNode>>): Promise<WorkflowTreeNode> {
  const cached = cache.get(file.uri);
  if (cached) {
    return cached;
  }
  const promise = Promise.resolve({ type: 'thinking', data: parseThinkingSession(file), children: [] } satisfies WorkflowTreeNode);
  cache.set(file.uri, promise);
  try {
    return await promise;
  } catch (error) {
    cache.delete(file.uri);
    throw error;
  }
}

async function buildWorkflowNode(api: WorkflowTreeMemoryApi, file: MemoryFile, workflowCache: Map<string, Promise<WorkflowTreeNode>>, thinkingCache: Map<string, Promise<WorkflowTreeNode>>): Promise<WorkflowTreeNode> {
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
        children.push(await buildThinkingNode(await resolveMemoryFile(api, 'sequential', step.submachineSessionId, file), thinkingCache));
      }
      for (const childWorkflowId of parsed.references.childWorkflowIdsByStep.get(step.stepNumber) ?? []) {
        const childNode = await buildWorkflowNode(api, await resolveMemoryFile(api, 'workflow', childWorkflowId, file), workflowCache, thinkingCache);
        if (childNode.type === 'workflow' && !firstChildWorkflowUri) {
          firstChildWorkflowUri = childNode.data.uri;
        }
        children.push(childNode);
      }
    }
    if (firstChildWorkflowUri) {
      parsed.session.childWorkflowUri = firstChildWorkflowUri;
    }
    return { type: 'workflow', data: parsed.session, children } satisfies WorkflowTreeNode;
  })();

  workflowCache.set(file.uri, promise);
  try {
    return await promise;
  } catch (error) {
    workflowCache.delete(file.uri);
    throw error;
  }
}

export async function buildWorkflowTree(api: WorkflowTreeMemoryApi, uri: string): Promise<WorkflowTreeNode> {
  const workflowCache = new Map<string, Promise<WorkflowTreeNode>>();
  const thinkingCache = new Map<string, Promise<WorkflowTreeNode>>();
  return buildWorkflowNode(api, await api.readMemory(uri), workflowCache, thinkingCache);
}
