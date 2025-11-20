using System.Text.Json;
using Maenifold.Utils;

#pragma warning disable CA1861, CA1805

namespace Maenifold.Tools
{
    public record ToolDescriptor(string Name, Func<JsonElement, object?> Invoke, string[]? Aliases = null, string? Description = null);

    public static class ToolRegistry
    {
        private static readonly Dictionary<string, ToolDescriptor> _byName = new(StringComparer.OrdinalIgnoreCase);
        private static bool _initialized = false;

        public static void Initialize()
        {
            if (_initialized) return;
            _initialized = true;

            void add(ToolDescriptor d)
            {
                _byName[d.Name] = d;
                if (d.Aliases != null)
                {
                    foreach (var a in d.Aliases)
                        _byName[a] = d;
                }
            }

            add(CreateWriteMemoryDescriptor());
            add(CreateReadMemoryDescriptor());
            add(CreateEditMemoryDescriptor());
            add(CreateDeleteMemoryDescriptor());
            add(CreateMoveMemoryDescriptor());
            add(CreateSearchMemoriesDescriptor());
            add(CreateExtractConceptsFromFileDescriptor());
            add(CreateSyncDescriptor());
            add(CreateBuildContextDescriptor());
            add(CreateVisualizeDescriptor());
            add(CreateSequentialThinkingDescriptor());
            add(CreateWorkflowDescriptor());
            add(CreateRecentActivityDescriptor());
            add(CreateMemoryStatusDescriptor());
            add(CreateGetConfigDescriptor());
            add(CreateGetHelpDescriptor());
            add(CreateListMemoriesDescriptor());
            add(CreateUpdateAssetsDescriptor());
            add(CreateRepairConceptsDescriptor());
            add(CreateAnalyzeConceptCorruptionDescriptor());
            add(CreateFindSimilarConceptsDescriptor());
            add(CreateStartWatcherDescriptor());
            add(CreateStopWatcherDescriptor());
            add(CreateRunFullBenchmarkDescriptor());
            add(CreateAdoptDescriptor());
            add(CreateAssumptionLedgerDescriptor());
            add(CreateAddMissingH1Descriptor());
            add(CreateListAssetsDescriptor());
            add(CreateReadMcpResourceDescriptor());
        }

        private static ToolDescriptor CreateWriteMemoryDescriptor() =>
            new("WriteMemory", payload =>
            {
                var title = PayloadReader.GetString(payload, "title");
                var content = PayloadReader.GetString(payload, "content");
                string? folder = null;
                if (payload.TryGetProperty("folder", out var f)) folder = f.GetString();
                var tags = PayloadReader.GetStringArray(payload, "tags");
                bool learn = false;
                if (payload.TryGetProperty("learn", out var learnProp))
                    learn = learnProp.GetBoolean();
                return MemoryTools.WriteMemory(title, content, string.IsNullOrEmpty(folder) ? null : folder, tags, learn);
            }, new[] { "writememory" }, "Create a memory file");

        private static ToolDescriptor CreateReadMemoryDescriptor() =>
            new("ReadMemory", payload =>
            {
                var identifier = PayloadReader.GetString(payload, "identifier");
                bool includeChecksum = PayloadReader.GetBool(payload, "includeChecksum", true);
                bool learn = false;
                if (payload.TryGetProperty("learn", out var learnProp))
                    learn = learnProp.GetBoolean();
                return MemoryTools.ReadMemory(identifier, includeChecksum, learn);
            }, new[] { "readmemory" }, "Read a memory file");

        private static ToolDescriptor CreateEditMemoryDescriptor() =>
            new("EditMemory", payload =>
            {
                var identifier = PayloadReader.GetString(payload, "identifier");
                var operation = PayloadReader.GetString(payload, "operation");
                var content = PayloadReader.GetString(payload, "content");
                string? checksum = null;
                if (payload.TryGetProperty("checksum", out var cs)) checksum = cs.GetString();
                string? findText = null;
                if (payload.TryGetProperty("findText", out var ft)) findText = ft.GetString();
                string? sectionName = null;
                if (payload.TryGetProperty("sectionName", out var sn)) sectionName = sn.GetString();
                int? expectedCount = null;
                if (payload.TryGetProperty("expectedCount", out var ec) && ec.ValueKind == JsonValueKind.Number) expectedCount = ec.GetInt32();
                bool learn = false;
                if (payload.TryGetProperty("learn", out var learnProp))
                    learn = learnProp.GetBoolean();
                return MemoryTools.EditMemory(identifier, operation, content, checksum, findText, sectionName, expectedCount, learn);
            }, new[] { "editmemory" }, "Edit a memory file");

        private static ToolDescriptor CreateDeleteMemoryDescriptor() =>
            new("DeleteMemory", payload =>
            {
                var identifier = PayloadReader.GetString(payload, "identifier");
                bool confirm = PayloadReader.GetBool(payload, "confirm", false);
                bool learn = false;
                if (payload.TryGetProperty("learn", out var learnProp))
                    learn = learnProp.GetBoolean();
                return MemoryTools.DeleteMemory(identifier, confirm, learn);
            }, new[] { "deletememory" }, "Delete a memory file");

        private static ToolDescriptor CreateMoveMemoryDescriptor() =>
            new("MoveMemory", payload =>
            {
                var source = PayloadReader.GetString(payload, "source");
                var destination = PayloadReader.GetString(payload, "destination");
                return MemoryTools.MoveMemory(source, destination);
            }, new[] { "movememory" }, "Move/rename a memory file");

        private static ToolDescriptor CreateSearchMemoriesDescriptor() =>
            new("SearchMemories", payload =>
            {
                var query = PayloadReader.GetString(payload, "query");
                var mode = payload.TryGetProperty("mode", out var pm) && pm.ValueKind == JsonValueKind.String
                    ? Enum.Parse<SearchMode>(pm.GetString()!, true)
                    : SearchMode.Hybrid;
                var pageSize = PayloadReader.GetInt32(payload, "pageSize", 10);
                var page = PayloadReader.GetInt32(payload, "page", 1);
                string? folder = null;
                if (payload.TryGetProperty("folder", out var sf)) folder = sf.GetString();
                var tags = PayloadReader.GetStringArray(payload, "tags");
                var minScore = PayloadReader.GetDouble(payload, "minScore", 0.0);
                bool learn = false;
                if (payload.TryGetProperty("learn", out var learnProp))
                    learn = learnProp.GetBoolean();
                return MemorySearchTools.SearchMemories(query, mode, pageSize, page, folder, tags, minScore, learn);
            }, new[] { "searchmemories" }, "Search memories");

        private static ToolDescriptor CreateExtractConceptsFromFileDescriptor() =>
            new("ExtractConceptsFromFile", payload =>
            {
                var identifier = PayloadReader.GetString(payload, "identifier");
                bool learn = false;
                if (payload.TryGetProperty("learn", out var learnProp))
                    learn = learnProp.GetBoolean();
                return MemoryTools.ExtractConceptsFromFile(identifier, learn);
            }, new[] { "extractconceptsfromfile" }, "Extract wiki link concepts from a file");

        private static ToolDescriptor CreateSyncDescriptor() =>
            new("Sync", payload =>
            {
                bool learn = false;
                if (payload.TryGetProperty("learn", out var learnProp))
                    learn = learnProp.GetBoolean();
                return GraphTools.Sync(learn);
            }, new[] { "sync" }, "Sync graph");

        private static ToolDescriptor CreateBuildContextDescriptor() =>
            new("BuildContext", payload =>
            {
                var conceptName = PayloadReader.GetString(payload, "conceptName");
                var depth = PayloadReader.GetInt32(payload, "depth", 2);
                var maxEntities = PayloadReader.GetInt32(payload, "maxEntities", 20);
                var includeContent = PayloadReader.GetBool(payload, "includeContent", false);
                return (object)GraphTools.BuildContext(conceptName, depth, maxEntities, includeContent);
            }, new[] { "buildcontext" }, "Build context around a concept");

        private static ToolDescriptor CreateVisualizeDescriptor() =>
            new("Visualize", payload =>
            {
                var conceptName = PayloadReader.GetString(payload, "conceptName");
                var depth = PayloadReader.GetInt32(payload, "depth", 2);
                var maxNodes = PayloadReader.GetInt32(payload, "maxNodes", 30);
                bool learn = false;
                if (payload.TryGetProperty("learn", out var learnProp))
                    learn = learnProp.GetBoolean();
                return GraphTools.Visualize(conceptName, depth, maxNodes, learn);
            }, new[] { "visualize" }, "Visualize concept graph");

        private static ToolDescriptor CreateSequentialThinkingDescriptor() =>
            new("SequentialThinking", payload =>
            {
                // Make response optional for cancel operations
                string? response = null;
                if (payload.TryGetProperty("response", out var r)) response = r.GetString();

                var nextThoughtNeeded = PayloadReader.GetBool(payload, "nextThoughtNeeded", false);
                var thoughtNumber = PayloadReader.GetInt32(payload, "thoughtNumber", 0);
                var totalThoughts = PayloadReader.GetInt32(payload, "totalThoughts", 0);
                string? sessionId = null;
                if (payload.TryGetProperty("sessionId", out var s)) sessionId = s.GetString();
                var cancel = PayloadReader.GetBool(payload, "cancel", false);
                string? thoughts = null;
                if (payload.TryGetProperty("thoughts", out var t)) thoughts = t.GetString();
                var isRevision = PayloadReader.GetBool(payload, "isRevision", false);
                int? revisesThought = null;
                if (payload.TryGetProperty("revisesThought", out var rev) && rev.ValueKind == JsonValueKind.Number) revisesThought = rev.GetInt32();
                int? branchFromThought = null;
                if (payload.TryGetProperty("branchFromThought", out var b) && b.ValueKind == JsonValueKind.Number) branchFromThought = b.GetInt32();
                string? branchId = null;
                if (payload.TryGetProperty("branchId", out var bid)) branchId = bid.GetString();
                var needsMoreThoughts = PayloadReader.GetBool(payload, "needsMoreThoughts", false);
                string? analysisType = null;
                if (payload.TryGetProperty("analysisType", out var at)) analysisType = at.GetString();
                string? parentWorkflowId = null;
                if (payload.TryGetProperty("parentWorkflowId", out var pw)) parentWorkflowId = pw.GetString();
                string? conclusion = null;
                if (payload.TryGetProperty("conclusion", out var c)) conclusion = c.GetString();
                bool learn = false;
                if (payload.TryGetProperty("learn", out var learnProp))
                    learn = learnProp.GetBoolean();

                return SequentialThinkingTools.SequentialThinking(response, nextThoughtNeeded, thoughtNumber, totalThoughts, sessionId, cancel, thoughts, isRevision, revisesThought, branchFromThought, branchId, needsMoreThoughts, analysisType, parentWorkflowId, conclusion, learn);
            }, new[] { "sequentialthinking" }, "Sequential thinking tool");

        private static ToolDescriptor CreateWorkflowDescriptor() =>
            new("Workflow", payload =>
            {
                string? sessionId = null;
                if (payload.TryGetProperty("sessionId", out var s)) sessionId = s.GetString();

                // Support both string and array formats for workflowId
                var workflowIds = PayloadReader.GetStringArray(payload, "workflowId");
                string? workflowId = null;

                if (workflowIds != null && workflowIds.Length > 0)
                {
                    // Pass array as JSON to Start method for sequential queueing
                    if (workflowIds.Length > 1)
                    {
                        workflowId = JsonSerializer.Serialize(workflowIds);
                    }
                    else
                    {
                        workflowId = workflowIds[0];
                    }
                }
                else if (payload.TryGetProperty("workflowId", out var w) && w.ValueKind == JsonValueKind.String)
                {
                    // Fall back to single string format for backward compatibility
                    workflowId = w.GetString();
                }

                string? response = null;
                if (payload.TryGetProperty("response", out var r)) response = r.GetString();
                string? thoughts = null;
                if (payload.TryGetProperty("thoughts", out var th)) thoughts = th.GetString();
                string? status = null;
                if (payload.TryGetProperty("status", out var st)) status = st.GetString();
                string? conclusion = null;
                if (payload.TryGetProperty("conclusion", out var cl)) conclusion = cl.GetString();
                var view = PayloadReader.GetBool(payload, "view", false);
                string? append = null;
                if (payload.TryGetProperty("append", out var ap)) append = ap.GetString();
                bool learn = false;
                if (payload.TryGetProperty("learn", out var learnProp))
                    learn = learnProp.GetBoolean();

                return WorkflowTools.Workflow(sessionId, workflowId, response, thoughts, status, conclusion, view, append, learn);
            }, new[] { "workflow" }, "Workflow tool");

        private static ToolDescriptor CreateRecentActivityDescriptor() =>
            new("RecentActivity", payload =>
            {
                var limit = PayloadReader.GetInt32(payload, "limit", 10);
                string? filter = null;
                if (payload.TryGetProperty("filter", out var f)) filter = f.GetString();
                TimeSpan? timespan = null;
                if (payload.TryGetProperty("timespan", out var ts) && TimeSpan.TryParse(ts.GetString(), out var parsed)) timespan = parsed;
                var includeContent = PayloadReader.GetBool(payload, "includeContent", false);
                bool learn = false;
                if (payload.TryGetProperty("learn", out var learnProp))
                    learn = learnProp.GetBoolean();
                return RecentActivityTools.RecentActivity(limit, filter, timespan, includeContent, learn);
            }, new[] { "recentactivity" }, "Recent activity");

        private static ToolDescriptor CreateMemoryStatusDescriptor() =>
            new("MemoryStatus", payload =>
            {
                bool learn = false;
                if (payload.TryGetProperty("learn", out var learnProp))
                    learn = learnProp.GetBoolean();
                return SystemTools.MemoryStatus(learn);
            }, new[] { "memorystatus" }, "Memory status");

        private static ToolDescriptor CreateGetConfigDescriptor() =>
            new("GetConfig", payload =>
            {
                bool learn = false;
                if (payload.TryGetProperty("learn", out var learnProp))
                    learn = learnProp.GetBoolean();
                return SystemTools.GetConfig(learn);
            }, new[] { "getconfig" }, "Get config");

        private static ToolDescriptor CreateGetHelpDescriptor() =>
            new("GetHelp", payload =>
            {
                string toolName = "";
                if (payload.TryGetProperty("toolName", out var tn)) toolName = tn.GetString() ?? "";
                return SystemTools.GetHelp(toolName);
            }, new[] { "gethelp" }, "Get help for a tool");

        private static ToolDescriptor CreateListMemoriesDescriptor() =>
            new("ListMemories", payload =>
            {
                string? path = null;
                if (payload.TryGetProperty("path", out var p)) path = p.GetString();
                bool learn = false;
                if (payload.TryGetProperty("learn", out var learnProp))
                    learn = learnProp.GetBoolean();
                return SystemTools.ListMemories(path, learn);
            }, new[] { "listmemories" }, "List memories");

        private static ToolDescriptor CreateUpdateAssetsDescriptor() =>
            new("UpdateAssets", payload =>
            {
                bool dryRun = PayloadReader.GetBool(payload, "dryRun", true);
                bool learn = false;
                if (payload.TryGetProperty("learn", out var learnProp))
                    learn = learnProp.GetBoolean();
                return SystemTools.UpdateAssets(dryRun, learn);
            }, new[] { "updateassets" }, "Update assets from package");

        private static ToolDescriptor CreateRepairConceptsDescriptor() =>
            new("RepairConcepts", payload =>
            {
                var concepts = PayloadReader.GetString(payload, "conceptsToReplace");
                var canonical = PayloadReader.GetString(payload, "canonicalConcept");
                string? folder = null;
                if (payload.TryGetProperty("folder", out var f)) folder = f.GetString();
                bool dryRun = PayloadReader.GetBool(payload, "dryRun", true);
                bool createWikiLinks = PayloadReader.GetBool(payload, "createWikiLinks", false);
                var minSemantic = PayloadReader.GetDouble(payload, "minSemanticSimilarity", 0.7);
                bool learn = false;
                if (payload.TryGetProperty("learn", out var learnProp))
                    learn = learnProp.GetBoolean();
                return ConceptRepairTool.RepairConcepts(concepts, canonical, folder, dryRun, createWikiLinks, minSemantic, learn);
            }, new[] { "repairconcepts" }, "Repair concepts");

        private static ToolDescriptor CreateAnalyzeConceptCorruptionDescriptor() =>
            new("AnalyzeConceptCorruption", payload =>
            {
                var family = PayloadReader.GetString(payload, "conceptFamily");
                var max = PayloadReader.GetInt32(payload, "maxResults", 50);
                bool learn = false;
                if (payload.TryGetProperty("learn", out var learnProp))
                    learn = learnProp.GetBoolean();
                return ConceptRepairTool.AnalyzeConceptCorruption(family, max, learn);
            }, new[] { "analyzeconceptcorruption" }, "Analyze concept corruption");

        private static ToolDescriptor CreateFindSimilarConceptsDescriptor() =>
            new("FindSimilarConcepts", payload =>
            {
                var name = PayloadReader.GetString(payload, "conceptName", required: false);
                var max = PayloadReader.GetInt32(payload, "maxResults", 10);
                bool learn = false;
                if (payload.TryGetProperty("learn", out var learnProp))
                    learn = learnProp.GetBoolean();
                return VectorSearchTools.FindSimilarConcepts(name, max, learn);
            }, new[] { "findsimilarconcepts" }, "Find similar concepts");

        private static ToolDescriptor CreateStartWatcherDescriptor() =>
            new("StartWatcher", payload =>
            {
                int? debounce = null;
                if (payload.TryGetProperty("debounceMs", out var d) && d.ValueKind == JsonValueKind.Number) debounce = d.GetInt32();
                return IncrementalSyncTools.StartWatcher(debounce);
            }, new[] { "startwatcher" }, "Start incremental sync watcher");

        private static ToolDescriptor CreateStopWatcherDescriptor() =>
            new("StopWatcher", _ => IncrementalSyncTools.StopWatcher(), new[] { "stopwatcher" }, "Stop incremental sync watcher");

        private static ToolDescriptor CreateRunFullBenchmarkDescriptor() =>
            new("RunFullBenchmark", payload =>
            {
                var iterations = PayloadReader.GetInt32(payload, "iterations", 5);
                var maxFiles = PayloadReader.GetInt32(payload, "maxTestFiles", 1000);
                var includeDeep = PayloadReader.GetBool(payload, "includeDeepTraversal", true);
                bool learn = false;
                if (payload.TryGetProperty("learn", out var learnProp))
                    learn = learnProp.GetBoolean();
                return PerformanceBenchmark.RunFullBenchmark(iterations, maxFiles, includeDeep, learn);
            }, new[] { "runfullbenchmark" }, "Run full benchmark");

        private static ToolDescriptor CreateAdoptDescriptor() =>
            new("Adopt", payload =>
            {
                var type = PayloadReader.GetString(payload, "type");
                var identifier = PayloadReader.GetString(payload, "identifier");
                bool learn = false;
                if (payload.TryGetProperty("learn", out var learnProp))
                    learn = learnProp.GetBoolean();
                return AdoptTools.Adopt(type, identifier, learn).GetAwaiter().GetResult();
            }, new[] { "adopt" }, "Adopt assets");

        private static ToolDescriptor CreateAssumptionLedgerDescriptor() =>
            new("AssumptionLedger", payload =>
            {
                var action = PayloadReader.GetString(payload, "action");
                string? assumption = null;
                if (payload.TryGetProperty("assumption", out var a)) assumption = a.GetString();
                string? context = null;
                if (payload.TryGetProperty("context", out var c)) context = c.GetString();
                string? validationPlan = null;
                if (payload.TryGetProperty("validationPlan", out var v)) validationPlan = v.GetString();
                string? confidence = null;
                if (payload.TryGetProperty("confidence", out var cf)) confidence = cf.GetString();
                var concepts = PayloadReader.GetStringArray(payload, "concepts");
                string? uri = null;
                if (payload.TryGetProperty("uri", out var u)) uri = u.GetString();
                string? status = null;
                if (payload.TryGetProperty("status", out var st)) status = st.GetString();
                string? notes = null;
                if (payload.TryGetProperty("notes", out var n)) notes = n.GetString();
                bool learn = false;
                if (payload.TryGetProperty("learn", out var learnProp))
                    learn = learnProp.GetBoolean();

                return AssumptionLedgerTools.AssumptionLedger(action, assumption, context, validationPlan, confidence, concepts, uri, status, notes, learn);
            }, new[] { "assumptionledger" }, "Assumption ledger");

        private static ToolDescriptor CreateAddMissingH1Descriptor() =>
            new("AddMissingH1", payload =>
            {
                bool dryRun = PayloadReader.GetBool(payload, "dryRun", true);
                int limit = PayloadReader.GetInt32(payload, "limit", 100);
                string? folder = null;
                if (payload.TryGetProperty("folder", out var f)) folder = f.GetString();
                bool createBackups = PayloadReader.GetBool(payload, "createBackups", false);
                bool learn = false;
                if (payload.TryGetProperty("learn", out var learnProp))
                    learn = learnProp.GetBoolean();
                return MaintenanceTools.AddMissingH1(dryRun, limit, folder, createBackups, learn);
            }, new[] { "addmissingh1" }, "Add missing H1 headers to markdown files");

        private static ToolDescriptor CreateListAssetsDescriptor() =>
            new("ListAssets", payload =>
            {
                string? type = null;
                if (payload.ValueKind != JsonValueKind.Undefined && payload.TryGetProperty("type", out var t))
                {
                    type = t.GetString();
                }
                return McpResourceTools.ListAssets(type);
            },
                new[] { "listassets" }, "List available asset types or metadata for a type");

        private static ToolDescriptor CreateReadMcpResourceDescriptor() =>
            new("ReadMcpResource", payload =>
            {
                var uri = PayloadReader.GetString(payload, "uri");
                return McpResourceTools.ReadMcpResource(uri);
            }, new[] { "readmcpresource" }, "Read MCP resource by URI");

        public static bool TryInvoke(string name, JsonElement payload, out object? result)
        {
            if (!_initialized) Initialize();

            result = null;
            if (string.IsNullOrWhiteSpace(name)) return false;

            if (!_byName.TryGetValue(name, out var descriptor))
                return false;

            result = descriptor.Invoke(payload);
            return true;
        }
    }
}
