import { deriveServerHealth, getAuthSetupAction } from './palette-health';
import { getBareToolName } from './tool-categories';
import { getSynonymTargets, matchesSynonymTarget, normalizeSearchValue, scoreField } from './tool-palette-helpers';
import type { McpDiscoveredTool, McpServerDiscoveryResult, ServerHealthStatus } from './types';

export interface ToolPaletteCategory {
  id: string;
  label: string;
  description: string;
}

interface ToolPaletteIntentRule {
  exactNames?: string[];
  keywords?: string[];
  serverPrefixes?: string[];
}

interface ToolPaletteIntentInput {
  name: string | null;
  server: string;
  description: string;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

export interface ToolPaletteEntry {
  kind: 'tool' | 'server_state';
  key: string;
  namespacedName: string;
  toolName: string | null;
  displayName: string;
  description: string;
  server: string;
  status: 'ok' | 'error';
  error?: string | null;
  health: ServerHealthStatus;
  inputSchema: Record<string, unknown> | null;
}

export const TOOL_PALETTE_CATEGORIES: ToolPaletteCategory[] = [
  { id: 'memory', label: 'Memory & Knowledge', description: 'Knowledge graph, search, and reasoning' },
  { id: 'crm', label: 'Accounts & CRM', description: 'Pipeline, deals, and account teams' },
  { id: 'workplace', label: 'Workplace Intelligence', description: 'Email, meetings, and M365 data' },
  { id: 'docs', label: 'Documentation', description: 'Microsoft Learn and code samples' },
  { id: 'tasks', label: 'Task Management', description: 'Gate tasks and delivery pipeline' },
  { id: 'devops', label: 'DevOps & Code', description: 'GitHub, repos, and CI/CD' },
  { id: 'data', label: 'Data & Analytics', description: 'Power BI queries and reports' },
  { id: 'other', label: 'Other Tools', description: 'Uncategorized tools' },
];

const TOOL_PALETTE_INTENT_RULES: Record<string, ToolPaletteIntentRule> = {
  memory: {
    exactNames: [
      'writememory', 'readmemory', 'editmemory', 'deletememory', 'movememory', 'searchmemories', 'listmemories',
      'extractconceptsfromfile', 'buildcontext', 'findsimilarconcepts', 'visualize',
      'sequentialthinking', 'workflow', 'pipeline', 'assumptionledger', 'recentactivity', 'adopt',
      'getconfig', 'gethelp', 'memorystatus', 'updateassets', 'repairconcepts', 'analyzeconceptcorruption',
      'runfullbenchmark', 'listagents', 'acpexec', 'readmcpresource', 'listassets',
    ],
    keywords: ['memory', 'knowledge', 'graph', 'context', 'concept', 'assumption'],
    serverPrefixes: ['maenifold'],
  },
  crm: {
    exactNames: [
      'get_account_team', 'get_my_deals', 'get_my_hok_activities', 'get_my_milestones', 'open_msx_record',
      'get_pipeline_summary', 'suggest_top_opportunities', 'get_opportunity_details', 'search_opportunities',
      'search_my_accounts_opportunities', 'get_account_overview', 'find_opportunities_by_product',
      'get_territory_overview', 'get_opportunity_solutions', 'get_opportunity_distribution',
      'dataverse_query', 'dataverse_write', 'dataset_access',
    ],
    keywords: ['account', 'crm', 'deal', 'opportunity', 'milestone', 'territory', 'pipeline', 'dataverse'],
    serverPrefixes: ['msx-mcp'],
  },
  workplace: {
    exactNames: ['ask_work_iq', 'accept_eula', 'get_debug_link'],
    keywords: ['email', 'meeting', 'calendar', 'teams', 'work iq', 'workplace', 'm365'],
    serverPrefixes: ['workiq'],
  },
  docs: {
    exactNames: ['microsoft_docs_search', 'microsoft_docs_fetch', 'microsoft_code_sample_search'],
    keywords: ['docs', 'documentation', 'learn', 'reference', 'code sample'],
    serverPrefixes: ['microsoft-learn'],
  },
  tasks: {
    exactNames: ['status', 'append', 'pass', 'fail', 'history', 'sync'],
    keywords: ['gate', 'task'],
    serverPrefixes: ['gate'],
  },
  devops: {
    exactNames: [
      'get_workflow', 'get_workflow_run', 'get_workflow_job', 'download_workflow_run_artifact',
      'get_workflow_run_usage', 'get_workflow_run_logs_url', 'list_workflows', 'list_workflow_runs',
      'list_workflow_jobs', 'list_workflow_run_artifacts', 'get_commit', 'get_file_contents', 'get_job_logs',
      'issue_read', 'list_branches', 'list_commits', 'list_copilot_spaces', 'list_issues',
      'list_pull_requests', 'pull_request_read', 'search_code', 'search_issues', 'search_pull_requests',
      'search_repositories', 'search_users', 'get_copilot_space',
    ],
    keywords: ['github', 'repo', 'repository', 'commit', 'pull request', 'workflow', 'branch', 'issue'],
    serverPrefixes: ['github-mcp-server'],
  },
  data: {
    exactNames: ['executequery', 'discoverartifacts', 'generatequery', 'getsemanticmodelschema', 'getreportmetadata'],
    keywords: ['power bi', 'semantic model', 'dax', 'report', 'artifact', 'dataset', 'analytics'],
    serverPrefixes: ['powerbi'],
  },
};

function matchesIntentRule(
  entry: ToolPaletteIntentInput,
  rule: ToolPaletteIntentRule | undefined,
): boolean {
  if (!rule) return false;

  const normalizedBareName = normalizeSearchValue(getBareToolName(entry.name ?? entry.server));
  const normalizedName = normalizeSearchValue(entry.name ?? '');
  const normalizedDescription = normalizeSearchValue(entry.description);
  const normalizedServer = normalizeSearchValue(entry.server);
  const intentText = [normalizedBareName, normalizedName, normalizedDescription].filter(Boolean).join(' ');

  if (rule.exactNames?.some((name) => normalizedBareName === normalizeSearchValue(name))) return true;
  if (rule.keywords?.some((keyword) => intentText.includes(normalizeSearchValue(keyword)))) return true;
  return rule.serverPrefixes?.some((prefix) => normalizedServer.startsWith(normalizeSearchValue(prefix))) ?? false;
}

function createToolEntry(result: McpServerDiscoveryResult, tool: McpDiscoveredTool, hasStaleCache: boolean): ToolPaletteEntry {
  const serverHealth = deriveServerHealth(result, hasStaleCache);
  // Auth-pattern failures stay auth_required even when stale tools exist;
  // only non-auth errors with cached tools fall back to stale.
  const toolHealth =
    result.status === 'error' && hasStaleCache && serverHealth !== 'auth_required'
      ? 'stale'
      : serverHealth;

  return {
    kind: 'tool',
    key: tool.name,
    namespacedName: tool.name,
    toolName: tool.name,
    displayName: tool.name.startsWith(`${result.server}.`)
      ? tool.name.slice(result.server.length + 1)
      : getBareToolName(tool.name),
    description: tool.description ?? 'No description.',
    server: result.server,
    status: result.status,
    error: result.error,
    health: toolHealth,
    inputSchema: isRecord(tool.inputSchema) ? tool.inputSchema : null,
  };
}

function createServerStateEntry(result: McpServerDiscoveryResult, hasStaleCache: boolean): ToolPaletteEntry | null {
  if (result.status !== 'error' || result.tools.length > 0) {
    return null;
  }

  const health = deriveServerHealth(result, hasStaleCache);
  if (health === 'stale') {
    return null;
  }

  const action = getAuthSetupAction(result.server);
  const displayName = health === 'auth_required'
    ? action?.label ?? `${result.server} setup required`
    : `${result.server} unavailable`;
  const description = health === 'auth_required'
    ? action?.description ?? 'Complete setup to load tools from this server.'
    : result.error ?? 'Server discovery failed and no cached tools are available.';

  return {
    kind: 'server_state',
    key: `server-state:${result.server}`,
    namespacedName: `server-state:${result.server}`,
    toolName: null,
    displayName,
    description,
    server: result.server,
    status: result.status,
    error: result.error,
    health,
    inputSchema: null,
  };
}

export function buildToolPaletteEntries(
  serverResults: McpServerDiscoveryResult[],
  hasStaleCacheByServer: Record<string, boolean>,
): ToolPaletteEntry[] {
  return serverResults.flatMap((result) => {
    const hasStaleCache = hasStaleCacheByServer[result.server] ?? false;
    const toolEntries = result.tools
      .slice()
      .sort((left, right) => left.name.localeCompare(right.name))
      .map((tool) => createToolEntry(result, tool, hasStaleCache));
    const serverStateEntry = createServerStateEntry(result, hasStaleCache);
    return serverStateEntry ? [...toolEntries, serverStateEntry] : toolEntries;
  });
}

export function getPaletteEntrySearchFields(entry: ToolPaletteEntry): string[] {
  return [entry.displayName, entry.namespacedName, entry.description, entry.server, entry.error ?? ''];
}

export function scoredSearch<T>(
  items: T[],
  query: string,
  fields: (item: T) => string[],
): Array<{ item: T; score: number }> {
  const normalizedQuery = normalizeSearchValue(query);
  if (!normalizedQuery) return [];
  const synonymTargets = getSynonymTargets(normalizedQuery);

  return items
    .map((item) => {
      const itemFields = fields(item);
      const fieldScore = itemFields.reduce((best, field) => Math.max(best, scoreField(field, normalizedQuery)), 0);
      const synonymScore = synonymTargets.length > 0 && itemFields.some((field) =>
        synonymTargets.some((target) => matchesSynonymTarget(field, target)))
        ? 50
        : 0;

      return { item, score: Math.max(fieldScore, synonymScore) };
    })
    .filter((entry) => entry.score > 0)
    .sort((left, right) => right.score - left.score);
}

export function categorizeTools<T>(
  tools: T[],
  getIntentInput: (item: T) => ToolPaletteIntentInput,
): Array<{ category: ToolPaletteCategory; tools: T[] }> {
  return TOOL_PALETTE_CATEGORIES
    .map((category) => ({
      category,
      tools: tools.filter((tool) => {
        if (category.id === 'other') {
          return !TOOL_PALETTE_CATEGORIES
            .filter((entry) => entry.id !== 'other')
            .some((entry) => matchesIntentRule(getIntentInput(tool), TOOL_PALETTE_INTENT_RULES[entry.id]));
        }

        return matchesIntentRule(getIntentInput(tool), TOOL_PALETTE_INTENT_RULES[category.id]);
      }),
    }))
    .filter((entry) => entry.tools.length > 0);
}
