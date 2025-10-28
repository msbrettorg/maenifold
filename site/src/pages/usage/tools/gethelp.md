---
layout: ../../../layouts/DocsLayout.astro
---

# GetHelp

Retrieves comprehensive tool documentation from Maenifold's help file system for detailed usage guidance. This tool provides the foundation for [[Progressive Disclosure]] in the [[Tool Help System]] architecture, enabling AI agents to access complete parameter documentation, usage examples, and troubleshooting information for any Maenifold tool.

## When to Use This Tool

- When AI needs complete parameter documentation beyond brief tool descriptions
- For accessing usage examples and common patterns for any Maenifold tool
- When troubleshooting tool usage issues or understanding error messages
- For discovering detailed integration workflows between multiple tools
- When implementing complex [[Sequential Thinking]] or [[Workflow]] patterns
- For understanding [[Ma Protocol]] compliance requirements for tool usage
- When exploring the full capabilities of unfamiliar Maenifold tools

## Key Features

- **Complete Documentation Access**: Loads full help files from `/src/assets/usage/tools/{toolname}.md`
- **Error Handling with Discovery**: Lists all available tools when requested tool documentation not found
- **Progressive Disclosure Support**: Provides detailed layer 3 documentation as specified in [[Tool Help System]] architecture
- **Integration Guidance**: Shows how each tool connects with related Maenifold tools
- **Troubleshooting Resources**: Includes common error patterns and solutions
- **Usage Examples**: Provides JSON payload examples for CLI and MCP usage
- **Ma Protocol Documentation**: Explains compliance requirements and architectural principles

## Parameters

| Parameter | Type | Required | Description | Example |
|-----------|------|----------|-------------|---------|
| toolName | string | Yes | Name of the Maenifold tool to get help for | "WriteMemory", "SearchMemories", "BuildContext" |

## Usage Examples

### Getting Help for Memory Tools
```json
{
  "toolName": "WriteMemory"
}
```
Returns complete WriteMemory documentation including parameters, examples, and integration patterns.

### Getting Help for Graph Tools  
```json
{
  "toolName": "BuildContext"
}
```
Returns comprehensive BuildContext guide with concept traversal examples and relationship mapping.

### Getting Help for Thinking Tools
```json
{
  "toolName": "SequentialThinking"
}
```
Returns detailed SequentialThinking documentation with multi-agent collaboration patterns.

### Invalid Tool Name Handling
```json
{
  "toolName": "NonExistentTool"
}
```
Returns error message with complete list of 19 available tools that have documentation.

## Available Tools with Documentation

GetHelp provides documentation for all Maenifold tools:

**Memory Tools**: BuildContext, DeleteMemory, EditMemory, ExtractConceptsFromFile, MoveMemory, ReadMemory, SearchMemories, WriteMemory

**System Tools**: GetConfig, GetHelp, ListMemories, MemoryStatus, RecentActivity, Sync

**Thinking Tools**: SequentialThinking, Workflow, ListWorkflows  

**Visualization Tools**: Visualize

## Progressive Disclosure Architecture

GetHelp serves as Layer 3 in the three-tier [[Progressive Disclosure]] system:

1. **Layer 1**: 5-line tool descriptions in MCP attributes (purpose, context, parameters, integration, output)
2. **Layer 2**: Response hints using [[ToolResponse]] utility ("💡 Run GetHelp for complete documentation")  
3. **Layer 3**: Complete help files accessed through GetHelp tool

This architecture prevents cognitive overload while ensuring comprehensive information is always available.

## Integration with Other Tools

- **WriteMemory**: Reference GetHelp in response hints to guide users to documentation
- **SearchMemories**: Include GetHelp suggestions when users need tool usage help
- **BuildContext**: Link to GetHelp when exploring tool relationship concepts
- **SequentialThinking**: Use GetHelp to access tool documentation during multi-step analysis
- **Workflow**: Reference GetHelp for understanding tool capabilities in workflow steps

## CLI Usage

```bash
# Get help for any tool
dotnet run -- --tool gethelp --payload '{"toolName": "WriteMemory"}'

# List all available tools (use invalid name)
dotnet run -- --tool gethelp --payload '{"toolName": "ListTools"}'
```

## MCP Usage

GetHelp is fully integrated into MCP mode and can be called directly by AI agents during Maenifold operations.

## Troubleshooting

### Error: "No help file found for tool: [ToolName]"
**Cause**: Tool name doesn't match any existing help file  
**Solution**: Check the "Available tools with help" list returned in the error message

### Missing documentation for new tools
**Cause**: New tool added without corresponding help file  
**Solution**: Create help file following Ma Protocol (250 lines max) in `/src/assets/usage/tools/`

### Incomplete progressive disclosure
**Cause**: Tool response hints not directing users to GetHelp  
**Solution**: Ensure tool implementations use [[ToolResponse]] utility methods

## Meta-Documentation Principle

GetHelp exemplifies [[Meta-Documentation]] - a system component that documents itself and other system components. This creates the [[Bootstrap Cycle]] necessary for complete [[System Completeness]] where the help system can help users discover and use the help system itself.

## Ma Protocol Compliance

GetHelp follows Maenifold's Ma Protocol principles:
- **Simplicity**: Single responsibility for documentation retrieval
- **Real Files**: Accesses actual markdown files, no database abstraction  
- **Static Methods**: Direct file system access without dependency injection
- **Minimal Complexity**: Straightforward file loading with error handling
- **250-line Limit**: Help files encourage focused, coherent documentation

This tool completes the Maenifold help system architecture by providing comprehensive documentation access while maintaining discoverability through self-reference.