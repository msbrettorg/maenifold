/** Maps maenifold tool names to their category label and display icon. */
export const TOOL_CATEGORIES: Record<string, { label: string; icon: string }> = {
  WriteMemory: { label: 'Memory', icon: '📝' },
  ReadMemory: { label: 'Memory', icon: '📖' },
  EditMemory: { label: 'Memory', icon: '✏️' },
  DeleteMemory: { label: 'Memory', icon: '🗑️' },
  MoveMemory: { label: 'Memory', icon: '📦' },
  SearchMemories: { label: 'Memory', icon: '🔍' },
  ListMemories: { label: 'Memory', icon: '📋' },
  ExtractConceptsFromFile: { label: 'Memory', icon: '🏷️' },
  BuildContext: { label: 'Graph', icon: '🌐' },
  FindSimilarConcepts: { label: 'Graph', icon: '🔗' },
  Visualize: { label: 'Graph', icon: '📊' },
  Sync: { label: 'Graph', icon: '🔄' },
  AnalyzeConceptCorruption: { label: 'Graph', icon: '🩺' },
  RepairConcepts: { label: 'Graph', icon: '🔧' },
  SequentialThinking: { label: 'Workflow', icon: '🧠' },
  Workflow: { label: 'Workflow', icon: '⚙️' },
  Pipeline: { label: 'Workflow', icon: '🔀' },
  AssumptionLedger: { label: 'Workflow', icon: '📒' },
  RecentActivity: { label: 'Workflow', icon: '📅' },
  Adopt: { label: 'System', icon: '🎭' },
  GetHelp: { label: 'System', icon: '❓' },
  GetConfig: { label: 'System', icon: '⚙️' },
  MemoryStatus: { label: 'System', icon: '📊' },
  UpdateAssets: { label: 'System', icon: '🔄' },
  ListAssets: { label: 'System', icon: '📋' },
  ReadMcpResource: { label: 'System', icon: '📄' },
  StartWatcher: { label: 'System', icon: '👁️' },
  StopWatcher: { label: 'System', icon: '⏹️' },
  RunFullBenchmark: { label: 'System', icon: '🏃' },
};

/** Groups tool definitions from MCP into categories using TOOL_CATEGORIES. */
export function groupToolsByCategory(
  tools: { name: string; description?: string }[],
): { group: string; tools: { name: string; description?: string }[] }[] {
  const groups = new Map<string, { name: string; description?: string }[]>();

  for (const tool of tools) {
    const cat = TOOL_CATEGORIES[tool.name]?.label ?? 'Other';
    if (!groups.has(cat)) groups.set(cat, []);
    groups.get(cat)!.push(tool);
  }

  const order = ['Memory', 'Graph', 'Workflow', 'System', 'Other'];
  return order
    .filter((g) => groups.has(g))
    .map((g) => ({ group: g, tools: groups.get(g)! }));
}
