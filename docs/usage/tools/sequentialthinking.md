# Sequential Thinking

A detailed tool for dynamic and reflective problem-solving through thoughts.
This tool helps analyze problems through a flexible thinking process that can adapt and evolve.
Each thought can build on, question, or revise previous insights as understanding deepens.
REQUIRES [[concepts]] in responses/thoughts to build connections in the knowledge graph!

## When to Use This Tool
- Breaking down complex problems into steps
- Planning and design with room for revision
- Analysis that might need course correction
- Problems where the full scope might not be clear initially
- Problems that require a multi-step solution
- Tasks that need to maintain context over multiple steps
- Situations where irrelevant information needs to be filtered out
- Multi-agent collaboration on complex reasoning tasks

Key features:
- You can adjust total_thoughts up or down as you progress
- You can question or revise previous thoughts
- You can add more thoughts even after reaching what seemed like the end
- You can express uncertainty and explore alternative approaches
- Not every thought needs to build linearly - you can branch or backtrack
- Generates a solution hypothesis
- Verifies the hypothesis based on the Chain of Thought steps
- Repeats the process until satisfied
- Provides a correct answer
- File-based persistence for resuming sessions later
- Multi-agent collaboration: multiple agents can work on the same session simultaneously
- Agents should use branchId when collaborating to avoid conflicts
- Full revision history with timestamps and agent identification

Parameters explained:
- response: Your current thinking step, which can include:
  * Regular analytical steps
  * Revisions of previous thoughts
  * Questions about previous decisions
  * Realizations about needing more analysis
  * Changes in approach
  * Hypothesis generation
  * Hypothesis verification
- nextThoughtNeeded: True if you need more thinking, even if at what seemed like the end
- thoughtNumber: Current number in sequence (can go beyond initial total if needed)
- totalThoughts: Current estimate of thoughts needed (can be adjusted up/down)
- sessionId: Unique identifier for this thinking session (enables multi-agent collaboration on same problem)
- thoughts: Ambient/meta thoughts about the thinking process itself
- isRevision: A boolean indicating if this thought revises previous thinking
- revisesThought: If isRevision is true, which thought number is being reconsidered
- branchFromThought: If branching, which thought number is the branching point
- branchId: Identifier for the current branch (RECOMMENDED for multi-agent collaboration to prevent conflicts)
- needsMoreThoughts: If reaching end but realizing more thoughts needed

You should:
1. Start with an initial estimate of needed thoughts, but be ready to adjust
2. Feel free to question or revise previous thoughts
3. Don't hesitate to add more thoughts if your understanding evolves
4. Use the thoughts parameter to capture meta-observations about your thinking
5. Each session creates a markdown file for full history and collaboration

Multi-agent collaboration:
- Multiple agents can work on the same sessionId simultaneously
- Each agent's thoughts are tagged with their AGENT_ID
- Use branchId when multiple agents are exploring different solution paths
- Branches prevent agents from overwriting each other's work
- All thoughts remain in chronological order with full attribution

Parent-child linking:
- Use parentWorkflowId to link this session to a parent workflow
- Creates bidirectional reference (child→parent, parent→children)
- Parent must exist and be active
- Links stored as WikiLinks for Obsidian compatibility