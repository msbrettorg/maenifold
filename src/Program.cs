
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using System.Text.Json;
using ModelContextProtocol.Server;
using Maenifold.Tools;
using Maenifold.Utils;

AssetManager.InitializeAssets();
if (args.Contains("--mcp"))
{
    var builder = Host.CreateApplicationBuilder(args);

    builder.Logging
            .SetMinimumLevel(LogLevel.Warning)
            .AddConsole(options =>
            {
                options.LogToStandardErrorThreshold = LogLevel.Warning;
            });

    builder.Services
            .AddMcpServer()
            .WithStdioServerTransport()
            .WithToolsFromAssembly()
            .WithResourcesFromAssembly();

    var app = builder.Build();

    // Ensure required directories exist
    Config.EnsureDirectories();

    if (Config.EnableIncrementalSync)
    {
        IncrementalSyncTools.StartWatcher();
    }

    // Start asset hot-loading watcher (Wave 3: RTM-011 to RTM-016)
    // Wave 4: Pass MCP server for resource update notifications (RTM-017 to RTM-020)
    var mcpServer = app.Services.GetRequiredService<McpServer>();
    AssetWatcherTools.StartAssetWatcher(mcpServer);

    await app.RunAsync();
    return;
}

var toolIndex = Array.IndexOf(args, "--tool");
if (toolIndex < 0)
{
    PrintUsage();
    return;
}

var payloadIndex = Array.IndexOf(args, "--payload");
if (payloadIndex < 0 || toolIndex + 1 >= args.Length || payloadIndex + 1 >= args.Length)
{
    PrintUsage();
    return;
}

// Ensure required directories exist
Config.EnsureDirectories();

// T-CLI-JSON-001: RTM FR-8.1 - Detect --json flag for structured output
var useJsonOutput = args.Contains("--json");

var toolName = args[toolIndex + 1];
var payloadJson = args[payloadIndex + 1];

try
{
    var payload = JsonSerializer.Deserialize<JsonElement>(payloadJson, SafeJson.Options);

    // T-CLI-JSON-001: RTM FR-8.1, FR-8.5 - Set output context based on --json flag
    if (useJsonOutput)
    {
        OutputContext.Format = OutputFormat.Json;
    }

    if (ToolRegistry.TryInvoke(toolName, payload, out var result))
    {
        var output = result is string s ? s : result?.ToString() ?? "";
        System.Console.WriteLine(output);
    }
    else
    {
        // T-CLI-JSON-001: RTM FR-8.4 - Structured error for unknown tool
        if (useJsonOutput)
        {
            var errorResponse = JsonToolResponse.Fail("UNKNOWN_TOOL", $"Unknown tool: {toolName}");
            System.Console.WriteLine(errorResponse.ToJson());
        }
        else
        {
            System.Console.WriteLine($"Unknown tool: {toolName}");
        }
        return;
    }
}
catch (JsonException ex)
{
    // T-CLI-JSON-001: RTM FR-8.4 - Structured error for JSON parsing errors
    if (useJsonOutput)
    {
        var errorResponse = JsonToolResponse.Fail("INVALID_PAYLOAD", $"Error invoking {toolName}: {ex.Message}");
        System.Console.WriteLine(errorResponse.ToJson());
    }
    else
    {
        Console.WriteLine($"Error invoking {toolName}: {ex.Message}");
    }
    return;
}
catch (ArgumentException ex)
{
    // T-CLI-JSON-001: RTM FR-8.4 - Structured error for argument errors
    if (useJsonOutput)
    {
        var errorResponse = JsonToolResponse.Fail("INVALID_ARGUMENT", $"Error invoking {toolName}: {ex.Message}");
        System.Console.WriteLine(errorResponse.ToJson());
    }
    else
    {
        Console.WriteLine($"Error invoking {toolName}: {ex.Message}");
    }
    return;
}
catch (Exception ex)
{
    // T-CLI-JSON-001: RTM FR-8.4 - Structured error for general errors
    if (useJsonOutput)
    {
        var errorResponse = JsonToolResponse.Fail("INTERNAL_ERROR", $"Error invoking {toolName}: {ex.Message}");
        System.Console.WriteLine(errorResponse.ToJson());
    }
    else
    {
        Console.WriteLine($"Error invoking {toolName}: {ex.Message}");
    }
    return;
}

void PrintUsage()
{
    System.Console.WriteLine("maenifold CLI");
    System.Console.WriteLine();
    System.Console.WriteLine("Usage:");
    System.Console.WriteLine("  MCP Server mode:   maenifold --mcp");
    System.Console.WriteLine("  Direct tool mode:  maenifold --tool <name> --payload '<json>' [--json]");
    System.Console.WriteLine();
    System.Console.WriteLine("  (For development from source: dotnet run -- --tool <name> --payload '<json>')");
    System.Console.WriteLine();
    System.Console.WriteLine("Options:");
    System.Console.WriteLine("  --json             Output structured JSON instead of markdown (FR-8.1)");
    System.Console.WriteLine();
    System.Console.WriteLine("Examples:");
    System.Console.WriteLine("  maenifold --tool WriteMemory --payload '{\"title\":\"Test\",\"content\":\"Content with [[WikiLink]]\"}'");
    System.Console.WriteLine("  maenifold --tool SearchMemories --payload '{\"query\":\"authentication\",\"mode\":\"Hybrid\"}'");
    System.Console.WriteLine("  maenifold --tool BuildContext --payload '{\"conceptName\":\"Machine Learning\",\"depth\":2}'");
    System.Console.WriteLine();
    System.Console.WriteLine("Available tools:");
    System.Console.WriteLine("  Memory: WriteMemory, ReadMemory, EditMemory, DeleteMemory, MoveMemory");
    System.Console.WriteLine("          SearchMemories, ExtractConceptsFromFile, RecentActivity");
    System.Console.WriteLine("  Vector: FindSimilarConcepts");
    System.Console.WriteLine("  Graph:  Sync, BuildContext, Visualize");
    System.Console.WriteLine("  Think:  SequentialThinking");
    System.Console.WriteLine("  Flow:   Workflow");
    System.Console.WriteLine("  System: MemoryStatus, ListMemories, GetConfig, GetHelp");
    System.Console.WriteLine("  Sync:   StartWatcher, StopWatcher");
    System.Console.WriteLine("  Repair: RepairConcepts, AnalyzeConceptCorruption");
    System.Console.WriteLine("  MCP:    ListAssets, ReadMcpResource");
    System.Console.WriteLine("  Bench:  RunFullBenchmark");
}
