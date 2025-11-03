# ListWorkflows

## Purpose
Discover and explore available structured workflows for systematic problem-solving, methodological guidance, and complex orchestration across various domains and thinking approaches.

## When to Use This Tool
- **Workflow Discovery**: Find appropriate methodologies for your specific problem type or domain
- **Methodology Selection**: Choose between design thinking, critical analysis, reasoning approaches, or development frameworks
- **Process Planning**: Identify structured approaches before starting complex multi-step tasks
- **Capability Assessment**: See what systematic approaches are available in Maenifold
- **Tool Chain Planning**: Understand which workflows enable Workflow tool orchestration
- **Problem Classification**: Match your challenge to proven methodological frameworks

## Key Features
- **Complete Workflow Catalog**: Lists all 40+ available structured methodologies and frameworks
- **Metadata Extraction**: Shows workflow ID, name, emoji identifier, and short description for each workflow
- **JSON Response Format**: Returns structured data perfect for automated tool selection decisions  
- **Domain Coverage**: Spans design thinking, reasoning methodologies, software development, creative processes, and analytical frameworks
- **Integration Ready**: Provides exact workflow IDs needed for Workflow tool execution
- **Error Resilient**: Gracefully handles malformed workflow files while continuing to process valid ones

## Parameters
This tool requires no parameters - it returns a complete catalog of available workflows.

| Parameter | Type | Required | Description | Example |
|-----------|------|----------|-------------|---------|
| *None* | - | - | No parameters required | - |

## Usage Examples
### Basic Example
```json
{}
```
**Result**: Complete JSON array of all available workflows with their metadata.

### Expected Response Structure
```json
[
  {
    "id": "design-thinking",
    "name": "Design Thinking", 
    "emoji": "💡",
    "description": "Human-centered approach to innovation and problem-solving"
  },
  {
    "id": "critical-thinking",
    "name": "Critical Thinking",
    "emoji": "🔍", 
    "description": "Systematic evaluation, analysis, and logical assessment of information and arguments"
  }
]
```

## Workflow Categories Available

### **🎨 Design & Creative Methodologies**
- **design-thinking**: Human-centered innovation approach
- **divergent-thinking**: Generate multiple creative solutions  
- **convergent-thinking**: Refine and focus creative solutions
- **oblique-strategies**: Creative problem-solving through constraints
- **lateral-thinking**: Alternative perspective problem-solving
- **scamper**: Systematic creative modification technique

### **🔍 Analysis & Reasoning Methodologies** 
- **critical-thinking**: Systematic evaluation and logical assessment
- **strategic-thinking**: Long-term planning and competitive analysis
- **deductive-reasoning**: General principles to specific conclusions
- **inductive-reasoning**: Specific observations to general patterns
- **abductive-reasoning**: Best explanation hypothesis formation
- **higher-order-thinking**: Meta-cognitive analysis and synthesis

### **⚙️ Software Development & Engineering**
- **agentic-dev**: AI agent-assisted Simple, Lovable, Complete development
- **agentic-slc**: SLC methodology with anti-slop controls  
- **agile**: Iterative development with user feedback loops
- **sdlc**: Traditional Software Development Life Cycle
- **code-review**: Systematic code quality assessment
- **agentic-test**: AI-assisted testing and quality assurance

### **🧠 Collaborative & Social Methodologies**
- **world-cafe**: Large group collaborative conversation
- **parallel-thinking**: Six Thinking Hats simultaneous perspective exploration
- **sixhat**: Edward de Bono's Six Thinking Hats framework
- **socratic-dialogue**: Question-based learning and discovery
- **role-creation-workflow**: AI persona and perspective development

### **📊 Problem-Solving & Analysis**
- **polya-problem-solving**: Four-step mathematical problem-solving approach
- **data-thinking**: Data-driven analysis and decision making
- **lean-startup**: Build-measure-learn innovation cycle
- **provocative-operation**: Challenge assumptions through provocation
- **crta**: Comprehensive Root Task Analysis

### **🚀 Specialized & Advanced Workflows**
- **agentic-pm**: AI-assisted product management
- **agentic-research**: Systematic research and investigation
- **agentic-troubleshooting**: AI-guided problem diagnosis and resolution
- **memory-database-decoupling**: Architecture design for data separation
- **workflow-dispatch**: Multi-workflow orchestration and coordination

## Common Patterns
- **Methodology Selection**: Use ListWorkflows to see options → choose appropriate workflow → execute with Workflow tool
- **Problem Classification**: Review workflow descriptions to match your problem type to proven methodological frameworks
- **Multi-Methodology Approaches**: Identify complementary workflows for comprehensive analysis (e.g., critical-thinking → design-thinking → agile)
- **Domain Expertise**: Find specialized workflows for your field (engineering, design, research, management)
- **Orchestration Planning**: Understand which workflows support multi-agent coordination and complex orchestration

## Related Tools
- **Workflow**: Executes the workflows discovered through ListWorkflows - requires exact workflow ID from this tool's output
- **SequentialThinking**: Many workflows embed Sequential Thinking for deep analysis steps - called automatically when workflows specify `requiresEnhancedThinking: true`
- **SearchMemories**: Find previous workflow experiences and results to inform methodology selection
- **WriteMemory**: Preserve successful workflow combinations and methodology insights for future use

## Integration with Maenifold Ecosystem
- **[[Concept]] Extraction**: All workflow responses build the knowledge graph through [[WikiLink]] concepts
- **Persistent Sessions**: Workflow execution maintains state across interaction sessions
- **Tool Orchestration**: Workflows coordinate Maenifold tools (SearchMemories, WriteMemory, BuildContext)
- **Multi-Agent Coordination**: Advanced workflows support parallel agent dispatch and collaboration
- **Quality Gates**: Workflows include built-in validation checkpoints and guardrails

## Workflow Selection Guidance

### **For Complex Problems**
Choose workflows with `enhancedThinkingEnabled: true` such as:
- design-thinking (user-centered problems)
- critical-thinking (evaluation challenges) 
- strategic-thinking (planning challenges)
- agentic-dev (software development)

### **For Creative Challenges**
- divergent-thinking → convergent-thinking sequence
- oblique-strategies for constraint-based creativity
- lateral-thinking for alternative perspectives

### **For Technical Implementation**  
- agentic-dev for AI-assisted development
- agile for iterative user-centered development
- sdlc for comprehensive system development

### **For Collaborative Decision-Making**
- world-cafe for large group input
- sixhat for comprehensive perspective analysis
- socratic-dialogue for deep understanding

## Troubleshooting
- **"ERROR: Workflows directory not found"** → Check Maenifold installation and assets/workflows directory exists
- **Empty Result Array []** → No valid workflow files found - verify JSON format in assets/workflows directory  
- **Missing Workflow Descriptions** → Some workflow files may lack shortDescription property - workflow will show "No description available"
- **Malformed JSON Skipped** → Invalid workflow files are silently skipped but valid workflows continue to load
- **Workflow ID Needed for Execution** → Use the exact "id" field from ListWorkflows output in the Workflow tool workflowId parameter

## Advanced Usage Patterns
- **Methodology Chaining**: Use multiple complementary workflows in sequence for comprehensive analysis
- **Problem-Workflow Mapping**: Build decision trees mapping problem types to optimal workflow selections  
- **Orchestration Planning**: Identify workflows supporting multi-agent coordination for complex systematic approaches
- **Quality Control**: Choose workflows with built-in guardrails and validation gates for high-stakes decisions

## Performance Characteristics
- **Fast Discovery**: Instant enumeration of all available methodological frameworks
- **Error Resilient**: Continues processing even if individual workflow files are malformed
- **Memory Efficient**: Loads only workflow metadata, not full step definitions  
- **Format Consistent**: Always returns JSON array suitable for automated processing

ListWorkflows serves as the discovery mechanism for Maenifold's comprehensive methodology catalog, enabling informed selection of structured approaches for systematic problem-solving and complex orchestration challenges.