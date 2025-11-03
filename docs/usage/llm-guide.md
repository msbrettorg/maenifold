# maenifold - LLM Usage Guide

## Overview

maenifold provides structured thinking frameworks (workflows) and organic exploration tools (SequentialThinking) to enhance your cognitive capabilities. This guide explains how to orchestrate these tools effectively.

## Core Concepts

### Workflows: Structured Frameworks
- **Purpose**: Provide proven, step-by-step methodologies for specific thinking tasks
- **Nature**: Rigid, predefined steps with clear progression
- **Examples**: critical-thinking (6 steps), design-thinking (5 steps), strategic-thinking (7 steps)
- **When to use**: When you need a systematic approach to a problem

### Sequential Thinking: Organic Exploration
- **Purpose**: Enable deep, flexible thinking with revision and branching capabilities
- **Nature**: Dynamic, allows backtracking, questioning assumptions, exploring alternatives
- **When to use**: When a workflow step requires deep exploration (requiresEnhancedThinking=true)

## Orchestration Pattern

The key insight: Workflows and SequentialThinking are **complementary opposites**. Workflows provide structure, while SequentialThinking provides depth at specific points.

### Step-by-Step Orchestration

1. **Start a workflow** by calling Workflow:
```json
{
  "step": 1,
  "stepContent": "Beginning analysis of the problem",
  "needsNextStep": false
}
```

2. **Check the response** for enhanced thinking requirements:
```json
{
  "currentStep": {
    "name": "Identify Assumptions",
    "requiresEnhancedThinking": true,
    "thinkingPrompt": "What assumptions are we making? Question everything."
  }
}
```

3. **If requiresEnhancedThinking=true**, pause the workflow and invoke SequentialThinking using the thinkingPrompt as guidance:
```json
{
  "thought": "[Start with or incorporate the thinkingPrompt] What assumptions are we making? Let me identify the core assumptions in this problem. First, I'm assuming that...",
  "contextId": "workflow-critical-thinking-step-2",
  "thoughtNumber": 1,
  "totalThoughts": 3,
  "nextThoughtNeeded": true
}
```

4. **Continue sequential thinking** until you've explored the step thoroughly:
```json
{
  "thought": "Actually, I need to revise my earlier assumption because...",
  "contextId": "workflow-critical-thinking-step-2",
  "thoughtNumber": 2,
  "totalThoughts": 4,
  "isRevision": true,
  "revisesThought": 1,
  "nextThoughtNeeded": true
}
```

5. **After completing sequential thinking**, synthesize your insights and continue the workflow:
```json
{
  "workflow": "critical-thinking",
  "step": 2,
  "stepContent": "After deep analysis, I've identified three key assumptions: [synthesized insights from sequential thinking]",
  "needsNextStep": true
}
```

## Important Orchestration Rules

1. **Use contextId for isolation**: When using sequential thinking for a workflow step, use `contextId: "workflow-{workflowId}-step-{stepNumber}"` to keep thinking sessions separate.

2. **The LLM is the orchestrator**: The server tools don't know about each other. You must decide when to use sequential thinking based on the `requiresEnhancedThinking` flag.

3. **Use thinkingPrompt as guidance**: When a workflow step provides a `thinkingPrompt`, use it to guide your sequential thinking. Start with the prompt or incorporate its direction into your first thought. The prompt provides intelligent guidance while still allowing organic exploration.

4. **Synthesize before continuing**: After sequential thinking, synthesize your insights into a clear summary for the workflow's `stepContent`.

5. **Respect the structure**: Workflows are meant to be rigid. Don't skip steps or change their order. Use sequential thinking for depth, not to bypass the structure.

## Example: Critical Thinking Workflow

```
1. Call Workflow → Get step 1 (Define Problem)
2. Step has requiresEnhancedThinking=false → Provide stepContent directly
3. Call Workflow → Get step 2 (Identify Assumptions)
4. Step has requiresEnhancedThinking=true → Call SequentialThinking
5. Think deeply with 3-5 thoughts, maybe revise some
6. Synthesize insights → Continue workflow with rich stepContent
7. Repeat for remaining steps
```

## The thinkingPrompt Pattern

When a workflow step includes a `thinkingPrompt`, this is "smart data" - intelligent guidance encoded in the workflow definition. Use it to:

1. **Set initial direction**: Begin your first sequential thought by addressing the prompt's question or challenge
2. **Maintain focus**: Let the prompt guide your exploration while remaining open to discoveries
3. **Explore freely**: The prompt is a starting point, not a constraint - follow interesting threads

Example from Workflow:
```
Step: "Analyze Problem Characteristics"
thinkingPrompt: "Analyze this problem systematically: What type of problem is this? Is it logical, creative, analytical, strategic, or complex/multi-faceted? What domain knowledge is required? What are the key constraints and success criteria? What makes this problem challenging?"
```

Your first thought should engage with these questions while maintaining freedom to explore.

## Tips for Effective Use

- **Don't rush through workflows**: Each step exists for a reason
- **Use sequential thinking contextId**: Keep different thinking sessions isolated
- **Let thinkingPrompts guide you**: They contain expert knowledge about what to explore
- **Branch when needed**: Sequential thinking supports branching for exploring alternatives
- **Revise freely**: In sequential thinking, you can always reconsider earlier thoughts
- **Trust the frameworks**: Workflows encode proven methodologies - follow them fully

## Thinking Tools Overview

maenifold provides two powerful thinking tools that embody different approaches to structured cognition:

### SequentialThinking Tool - Sequential Thinking with Persistence

The `SequentialThinking` tool provides structured yet flexible thinking with automatic session persistence and knowledge graph integration.

**When to use SequentialThinking:**
- Breaking down complex problems with evolving understanding
- Building connected knowledge while thinking
- Analysis requiring revision and branching
- When workflow steps show `requiresEnhancedThinking=true`
- Creating referenceable thinking sessions

**Key features:**
- Persistent storage in memory://thinking/
- Automatic relations extraction from [[WikiLinks]]
- Revision tracking (isRevision, revisesThought)
- Branch support for exploring alternatives
- Flexible thought count adjustment

**Example:**
```json
{
  "response": "The memory leak suggests event listeners aren't being cleaned up properly. Let me trace the [[Connection Lifecycle]].",
  "thoughtNumber": 1,
  "totalThoughts": 4,
  "nextThoughtNeeded": true,
  "thoughts": "User mentioned it happens even with setMaxListeners(30), so it's definitely a leak not just a warning"
}
```

### Workflow Tool - Structured Methodology Execution

The `Workflow` tool executes predefined workflows step by step with automatic persistence to the memory system.

**When to use Workflow:**
- Following structured problem-solving methodologies
- Executing predefined workflows (critical-thinking, design-thinking, etc.)
- When you need guided step-by-step analysis
- Chaining multiple workflows together

**Key features:**
- Sequential step execution through 28 predefined workflows
- Automatic persistence to memory://thinking/
- WikiLinks extraction for knowledge graph building
- Support for workflow chaining (execute multiple workflows in sequence)
- Ambient thoughts capture at each step

**Example:**
```json
{
  "workflowId": "critical-thinking",
  "response": "The problem is clearly defined as optimizing the [[Memory System]] performance under high load",
  "thoughts": "User mentioned 1000+ files, so scale is a key consideration"
}
```

### Key Differences

| Aspect | SequentialThinking | Workflow |
|--------|-------------------|----------|
| Structure | Flexible thinking | Rigid workflow steps |
| Use case | Exploration & analysis | Structured methodologies |
| Control | You control the flow | Workflow controls the flow |
| Persistence | Saves as thinking sessions | Saves as workflow execution |
| Best for | Open-ended problems | Well-defined processes |

### The Thoughts Parameter

Both tools support a `thoughts` parameter for capturing ambient insights:
- Meta-observations about the process
- Uncertainties or questions for later analysis
- Alternative approaches considered
- Contextual insights that emerge during thinking

This ensures all reasoning is captured, not just the structured outputs.

### Best Practices

1. **Choose the right tool**: SequentialThinking for exploration, Workflow for structured methodologies
2. **Use [[WikiLinks]]**: Connect concepts to build knowledge graph
3. **Capture thoughts**: Use the thoughts field for meta-insights
4. **Let complexity emerge**: Start simple, expand as needed
5. **Save important insights**: Both tools persist to the memory system