// Uses the same published @modelcontextprotocol/sdk package as toolWithSampleServer.ts.
// Run with: pnpm tsx multiTurnToolLoopServer.ts

import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { z } from 'zod';

type SamplingContentBlock =
    | { type: 'text'; text: string }
    | { type: 'tool_use'; id: string; name: string; input: Record<string, unknown> }
    | {
          type: 'tool_result';
          toolUseId: string;
          content: Array<{ type: 'text'; text: string }>;
      };

const mcpServer = new McpServer({
    name: 'multi-turn-tool-loop-server',
    version: '1.0.0'
});

const continueTool = {
    name: 'continue',
    description: 'continue the chain',
    inputSchema: {
        type: 'object' as const,
        properties: {
            note: {
                type: 'string',
                description: 'Optional note about why the chain should continue.'
            }
        }
    }
};

function normalizeContent(content: SamplingContentBlock | SamplingContentBlock[]) {
    return Array.isArray(content) ? content : [content];
}

function isToolUseContent(block: SamplingContentBlock): block is Extract<SamplingContentBlock, { type: 'tool_use' }> {
    return block.type === 'tool_use';
}

mcpServer.registerTool(
    'chain-of-tools',
    {
        description: 'Create a multi-turn sampling loop by returning tool_result blocks into the next createMessage call.',
        inputSchema: z.object({
            iterations: z.number().int().positive().default(15).describe('Number of sampling iterations to attempt.')
        })
    },
    async ({ iterations }) => {
        const totalIterations = iterations ?? 15;
        const messages: Array<{
            role: 'user' | 'assistant';
            content: SamplingContentBlock | SamplingContentBlock[];
        }> = [
            {
                role: 'user',
                content: {
                    type: 'text',
                    text: 'Use the provided continue tool exactly once per turn. After each tool result, call the continue tool again so the sampling request forms a multi-turn tool loop.'
                }
            }
        ];

        let completedIterations = 0;

        for (let iteration = 1; iteration <= totalIterations; iteration += 1) {
            process.stderr.write(`[multi-turn] iteration ${iteration}/${totalIterations}\n`);

            const response = await mcpServer.server.createMessage({
                messages,
                maxTokens: 300,
                tools: [continueTool],
                toolChoice: { mode: 'required' }
            });

            completedIterations = iteration;
            messages.push({
                role: 'assistant',
                content: response.content as SamplingContentBlock | SamplingContentBlock[]
            });

            const toolUses = normalizeContent(response.content as SamplingContentBlock | SamplingContentBlock[]).filter(
                isToolUseContent
            );

            if (toolUses.length === 0) {
                break;
            }

            messages.push({
                role: 'user',
                content: toolUses.map((toolUse) => ({
                    type: 'tool_result' as const,
                    toolUseId: toolUse.id,
                    content: [
                        {
                            type: 'text' as const,
                            text: `iteration ${iteration} acknowledged`
                        }
                    ]
                }))
            });
        }

        return {
            content: [
                {
                    type: 'text',
                    text: `Completed ${completedIterations} sampling iteration${completedIterations === 1 ? '' : 's'}.`
                }
            ]
        };
    }
);

async function main() {
    const transport = new StdioServerTransport();
    await mcpServer.connect(transport);
    process.stderr.write('Multi-turn MCP sampling server is running...\n');
}

try {
    await main();
} catch (error) {
    console.error('Server error:', error);
    // eslint-disable-next-line unicorn/no-process-exit
    process.exit(1);
}
