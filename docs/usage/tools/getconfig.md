# GetConfig

Display current system configuration settings and operational parameters to understand Maenifold's runtime environment, debug setup issues, and verify system paths. This tool provides essential configuration information including memory paths, database locations, synchronization settings, and performance parameters for troubleshooting and system validation.

## When to Use This Tool

- When troubleshooting Maenifold startup or runtime issues
- Before diagnosing path-related problems with memory files or database
- To verify environment variable configurations are properly loaded
- When setting up Maenifold in new environments or deployment contexts
- For debugging synchronization timing or performance issues
- To confirm system configuration before complex operations
- When providing system information for support or documentation
- To validate configuration consistency across different deployment environments

## Key Features

- **Path Resolution**: Shows resolved paths for memory directory, database, and assets
- **Environment Variable Display**: Reveals which settings come from environment overrides
- **Performance Parameters**: Displays timing, concurrency, and optimization settings
- **System Health Check**: Provides quick overview of critical configuration values
- **Debug Information**: Essential data for troubleshooting configuration issues
- **Ma Protocol Compliance**: Static configuration display with no side effects
- **Zero Dependencies**: Pure configuration read with no external system calls
- **Instant Response**: Immediate configuration summary without file system access

## Parameters

| Parameter | Type | Required | Description | Example |
|-----------|------|----------|-------------|---------|
| (none) | - | - | GetConfig takes no parameters - displays current system configuration | `{}` |

## Usage Examples

### Basic Configuration Display
```json
{}
```
Shows complete system configuration including all paths, timing settings, and operational parameters.

### System Troubleshooting
```json
{}
```
Use when Maenifold is not behaving as expected to verify configuration values and identify potential issues.

### Environment Validation
```json
{}
```
Run in new deployment environments to confirm all configuration values are correctly resolved and accessible.

## Configuration Categories

### Essential Paths
- **Memory Path**: Where knowledge files are stored and accessed
- **Database Path**: SQLite database location for concept graph storage
- **Assets Path**: Location of workflows, roles, and system resources

### Synchronization Settings
- **Debounce Timing**: File change detection delay for incremental sync
- **Auto Sync**: Whether automatic synchronization is enabled
- **Max Batch Size**: Limit for concurrent file operations

### Performance Parameters
- **Search Limits**: Default result counts for memory searches
- **Context Depth**: Default graph traversal depth for BuildContext
- **Connection Pooling**: Database connection management settings

### Debug Settings
- **Debug Logging**: Whether detailed debug information is enabled
- **Sync Logging**: File synchronization logging verbosity

## Configuration Sources

### Default Values
Maenifold provides sensible defaults for all configuration parameters based on common usage patterns and optimal performance characteristics.

### Environment Variable Overrides
All configuration values can be customized via environment variables:
- **MAENIFOLD_ROOT**: Base directory for all Maenifold data
- **MAENIFOLD_DATABASE_PATH**: Custom database file location
- **MAENIFOLD_DEBOUNCE_MS**: File change detection delay (default: 150)
- **MAENIFOLD_AUTO_SYNC**: Enable/disable automatic synchronization (default: true)
- **MAENIFOLD_MAX_BATCH_SIZE**: Maximum debounce batch size (default: 50)
- **MAENIFOLD_SEARCH_LIMIT**: Default search result limit (default: 10)
- **MAENIFOLD_CONTEXT_DEPTH**: Default context traversal depth (default: 2)
- **MAENIFOLD_MAX_ENTITIES**: Maximum context entities (default: 20)
- **MAENIFOLD_SNIPPET_LENGTH**: Recent activity snippet length (default: 1000)

### Platform-Specific Defaults
Configuration automatically adapts to platform conventions while allowing full customization through environment variables.

## Common Patterns

### Initial System Setup
Run GetConfig immediately after Maenifold installation to verify all paths are correctly resolved and accessible.

### Environment Migration
Use GetConfig when moving Maenifold between development, staging, and production environments to ensure configuration consistency.

### Troubleshooting Workflow
Always check configuration first when investigating Maenifold issues - many problems stem from incorrect path configuration.

### Performance Tuning
Review timing and performance parameters when optimizing Maenifold for specific workloads or system constraints.

### Documentation and Support
Include GetConfig output when reporting issues or documenting system setup for team knowledge sharing.

## Related Tools

- **MemoryStatus**: Shows runtime statistics that complement configuration information
- **ListMemories**: Verifies that configured paths contain expected content
- **Sync**: Relies on configuration paths for database and memory file operations
- **RecentActivity**: Uses configuration parameters for time-based filtering and display limits
- **All Tools**: Every Maenifold tool depends on configuration paths and settings

## Configuration Validation

### Path Accessibility
While GetConfig displays configured paths, it doesn't verify accessibility - use ListMemories or MemoryStatus to confirm paths are usable.

### Environment Variable Parsing
GetConfig shows the final resolved values after environment variable processing and type conversion.

### Default Behavior
When environment variables are not set, GetConfig displays the built-in default values that ensure Maenifold operates correctly.

### Cross-Platform Compatibility
Configuration automatically adapts to Windows, macOS, and Linux platform conventions for user directories and file paths.

## Troubleshooting

### Issue: "Configuration shows unexpected paths"
**Cause**: Environment variables may be overriding default locations  
**Solution**: Check environment variables with same names as configuration keys

### Issue: "Memory path points to wrong location"
**Cause**: MAENIFOLD_ROOT environment variable set incorrectly  
**Solution**: Verify environment variable or unset to use default: `~/maenifold`

### Issue: "Database path inaccessible"
**Cause**: MAENIFOLD_DATABASE_PATH points to read-only or non-existent directory  
**Solution**: Ensure database path directory exists and is writable

### Issue: "Auto sync disabled unexpectedly"
**Cause**: MAENIFOLD_AUTO_SYNC environment variable set to false  
**Solution**: Check environment variable or unset to enable automatic synchronization (default: true)

### Issue: "Performance settings seem wrong"
**Cause**: Environment variables overriding performance defaults  
**Solution**: Review all MAENIFOLD_* environment variables for unintended overrides

## System Configuration Architecture

### Static Configuration Pattern
GetConfig follows Ma Protocol principles with static configuration loading and no runtime dependency injection complexity.

### Environment-First Design
All configuration supports environment variable override while providing sensible defaults for immediate usability.

### Path Resolution Strategy
Configuration resolves paths relative to user home directory by default, with full override capability for custom deployments.

### Performance Optimization
Configuration values are loaded once at startup and cached statically for optimal performance across all tool operations.

## Integration Patterns

### Startup Validation
Many Maenifold deployment scripts include GetConfig verification to ensure proper configuration before operation.

### Error Context
When Maenifold tools report errors, GetConfig output provides essential context for diagnosing configuration-related issues.

### Team Collaboration
Teams often standardize Maenifold configuration through environment variables documented alongside GetConfig output.

### Monitoring Integration
System monitoring tools can call GetConfig to verify Maenifold configuration consistency across deployment environments.

## Ma Protocol Compliance

GetConfig exemplifies Maenifold's Ma Protocol philosophy:
- **Simplicity**: Pure configuration display with no side effects or complexity
- **Transparency**: Shows exactly what configuration values are active
- **No Magic**: Direct configuration access without abstraction layers
- **Static Behavior**: Consistent output based on startup configuration state
- **Zero Dependencies**: Self-contained configuration display requiring no external resources

This tool provides the foundational system information needed to understand, troubleshoot, and validate Maenifold configuration across all deployment contexts.