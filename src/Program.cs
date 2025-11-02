
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using System.Text.Json;
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

var toolName = args[toolIndex + 1];
var payloadJson = args[payloadIndex + 1];

var payload = JsonSerializer.Deserialize<JsonElement>(payloadJson);

if (ToolRegistry.TryInvoke(toolName, payload, out var result))
{
    var output = result is string s ? s : result?.ToString() ?? "";
    System.Console.WriteLine(output);
}
else
{
    System.Console.WriteLine($"Unknown tool: {toolName}");
    return;
}

void PrintUsage()
{
    System.Console.WriteLine("maenifold CLI");
    System.Console.WriteLine();
    System.Console.WriteLine("Usage:");
    System.Console.WriteLine("  MCP Server mode:   maenifold --mcp");
    System.Console.WriteLine("  Direct tool mode:  maenifold --tool <name> --payload '<json>'");
    System.Console.WriteLine();
    System.Console.WriteLine("  (For development from source: dotnet run -- --tool <name> --payload '<json>')");
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
    System.Console.WriteLine("  Flow:   Workflow, ListWorkflows");
    System.Console.WriteLine("  System: MemoryStatus, ListMemories, GetConfig, GetHelp");
    System.Console.WriteLine("  Sync:   StartWatcher, StopWatcher");
    System.Console.WriteLine("  Repair: RepairConcepts, AnalyzeConceptCorruption");
    System.Console.WriteLine("  Bench:  RunFullBenchmark");
}

PrintUsage();
