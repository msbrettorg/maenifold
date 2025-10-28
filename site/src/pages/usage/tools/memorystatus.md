---
layout: ../../../layouts/DocsLayout.astro
---

# MemoryStatus

Monitor Maenifold system health through comprehensive memory statistics and database metrics. This tool provides essential system monitoring capabilities to track knowledge graph growth, file storage utilization, and database health for system optimization and troubleshooting. Use for proactive system maintenance and performance analysis.

## When to Use This Tool

- Monitoring overall system health and resource utilization
- Tracking knowledge graph growth and concept relationship density
- Diagnosing performance issues or unexpected system behavior  
- Validating system integrity after bulk operations like Sync
- Planning system maintenance or optimization efforts
- Troubleshooting memory or database-related problems
- Generating system reports for analysis or documentation
- Monitoring storage usage before reaching capacity limits
- Verifying successful database initialization and graph building

## Key Features

- **File System Statistics**: Complete memory file counts, folder structure, and storage utilization
- **Database Health Monitoring**: Concept counts, relationship metrics, and database size tracking
- **Knowledge Graph Metrics**: Comprehensive statistics on concepts, relations, and mention frequencies
- **Storage Analysis**: Detailed size calculations for files and database components
- **Initialization Status**: Clear indication of database status and setup requirements
- **Performance Indicators**: Metrics useful for identifying system bottlenecks
- **Zero Dependencies**: Pure system metrics without external service requirements
- **Real-Time Accuracy**: Direct database queries ensure current, accurate statistics

## Parameters

| Parameter | Type | Required | Description | Example |
|-----------|------|----------|-------------|---------|
| *none* | - | - | MemoryStatus requires no parameters for operation | {} |

## Usage Examples

### Basic System Status Check
```json
{}
```
Returns comprehensive system statistics including file counts, storage usage, and complete database metrics.

### System Health Monitoring
```json
{}
```
Use regularly to monitor:
- Knowledge graph growth patterns
- Storage consumption trends  
- Database relationship density
- System performance indicators

### Post-Operation Validation
```json
{}
```
Run after bulk operations like:
- Large file imports or batch WriteMemory operations
- Sync operations to rebuild knowledge graph
- Database maintenance or optimization tasks
- System migrations or configuration changes

## Common Patterns

### System Health Monitoring
Use MemoryStatus as part of regular system maintenance routines. Monitor trends in concept growth, relationship density, and storage utilization to identify optimization opportunities.

### Performance Troubleshooting  
When system performance degrades, MemoryStatus provides baseline metrics to identify bottlenecks. Compare current statistics with historical data to isolate performance issues.

### Capacity Planning
Track storage growth patterns and database expansion rates. Use metrics to plan storage upgrades or implement data archival strategies before reaching capacity limits.

### Database Validation
After Sync operations or system changes, verify database integrity through concept counts and relationship metrics. Ensure expected graph structure matches actual database state.

### System Documentation
Include MemoryStatus output in system documentation, troubleshooting guides, and performance analysis reports to provide concrete baseline measurements.

## Related Tools

- **Sync**: Run after Sync to validate knowledge graph rebuilding and measure impact on system metrics
- **SearchMemories**: Use statistics to understand search performance and optimize query strategies  
- **WriteMemory**: Monitor storage growth patterns when creating large volumes of knowledge files
- **ListMemories**: Complement file system statistics with detailed directory structure analysis
- **RecentActivity**: Cross-reference system activity with health metrics for comprehensive monitoring

## System Metrics Explained

### File Statistics
- **Total files**: Count of all .md files in memory system
- **Total folders**: Directory count including nested structures
- **Total size**: Combined storage usage of all memory files in MB

### Graph Database Metrics  
- **Concepts**: Unique [[concept]] nodes extracted from memory files
- **Relations**: Connections between concepts within and across files
- **Mentions**: Total occurrences of concepts across all memory content
- **Database size**: SQLite database storage footprint in MB

### Health Indicators
- **High concept-to-file ratio**: Indicates rich knowledge interconnectivity
- **Balanced relations-to-concepts ratio**: Shows healthy graph density
- **Growing mention counts**: Demonstrates active knowledge reuse and linking

## Troubleshooting

### Database Not Initialized
**Status**: "Graph Database - Not initialized (run sync first)"  
**Cause**: SQLite database doesn't exist or is empty
**Solution**: Run Sync tool to extract concepts and build knowledge graph from existing memory files

### Zero Concepts Despite Having Files
**Status**: Files exist but zero concepts in database  
**Cause**: Files lack [[concept]] WikiLinks or Sync hasn't run
**Solution**: Add [[concepts]] to files using EditMemory, then run Sync to rebuild graph

### Large Database Size vs. Small File Count
**Status**: Database much larger than expected for file volume  
**Cause**: High relationship density or embedded vector storage
**Solution**: Normal for rich knowledge graphs; consider archival if growth excessive

### Performance Degradation with Growth
**Status**: System slowdown correlated with metric growth  
**Cause**: Database or file system performance bottlenecks
**Solution**: Analyze specific metrics to identify bottleneck (file I/O vs database queries)

### Inconsistent Metrics After Changes
**Status**: Statistics don't reflect recent operations  
**Cause**: Stale database state after bulk operations
**Solution**: Run Sync to refresh database and recalculate all metrics

## Performance Implications

### Resource Usage
MemoryStatus performs:
- File system traversal for .md files (I/O intensive for large directories)
- SQLite database queries (CPU intensive for large graphs)
- Size calculations (I/O intensive for storage metrics)

### Optimization Tips
- Run MemoryStatus during low-activity periods for large systems
- Use metrics to identify when database optimization or file reorganization needed
- Monitor trends rather than absolute values for meaningful insights

### Scalability Considerations
- Performance scales with file count and database size
- Large knowledge graphs (>10K concepts) may show slower query response
- Consider partitioning or archival strategies for very large systems

## Integration Patterns

### Automated Monitoring
Incorporate MemoryStatus into system monitoring scripts or health check routines. Set thresholds for storage usage and graph growth rates.

### Performance Analysis Workflows
Use MemoryStatus as first step in performance troubleshooting workflows. Establish baseline metrics before applying optimization strategies.

### System Documentation
Include MemoryStatus output in system status reports, capacity planning documents, and performance optimization analysis.

## Ma Protocol Compliance

MemoryStatus exemplifies Ma Protocol principles:
- **Simplicity**: Single responsibility for system health monitoring
- **No Magic**: Direct file system and database queries without caching complexity  
- **Minimal Complexity**: Static method with no parameters, clear output format
- **Real Testing**: Actual file counts and database metrics, no mock data
- **Transparent Operation**: Clear, readable output showing exactly what system contains

This tool provides essential visibility into your Maenifold system health, enabling proactive maintenance and optimization of your growing knowledge architecture.