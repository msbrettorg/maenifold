<!-- Minimal local copy of the official ACP docs. Source: https://agentclientprotocol.com/protocol/content -->

# Content

> Understanding content blocks in the Agent Client Protocol

Content blocks represent displayable information that flows through the Agent Client Protocol. They provide a structured way to handle various types of user-facing content—whether it's text from language models, images for analysis, or embedded resources for context.

Content blocks appear in:

* User prompts sent via `session/prompt`
* Language model output streamed through `session/update` notifications
* Progress updates and results from tool calls

## Content Types

The Agent Client Protocol uses the same `ContentBlock` structure as the Model Context Protocol (MCP).

### Text Content

Plain text messages form the foundation of most interactions.

All Agents **MUST** support text content blocks when included in prompts.

### Image Content

Images can be included for visual context or analysis.

Requires the `image` prompt capability when included in prompts.

### Audio Content

Audio data for transcription or analysis.

Requires the `audio` prompt capability when included in prompts.

### Embedded Resource

Complete resource contents embedded directly in the message.

This is the preferred way to include context in prompts, such as when using @-mentions to reference files or other resources.

Requires the `embeddedContext` prompt capability when included in prompts.

### Resource Link

References to resources that the Agent can access.
