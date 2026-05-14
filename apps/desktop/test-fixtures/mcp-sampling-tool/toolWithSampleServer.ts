// Lifted from modelcontextprotocol/typescript-sdk@2c0c481cb9dbfd15c8613f765c940a5f5bace94d — MIT
// Adapted to use the published @modelcontextprotocol/sdk@^1.29.0 because
// @modelcontextprotocol/server@2.0.0-alpha.{1,2} (the package the source
// repo's workspace points at) ships without `./stdio` in its npm `exports`
// map. See T-303.4.24 closure for full reproduction. Same wire behavior.
// Run with: pnpm tsx toolWithSampleServer.ts

import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { z } from 'zod';

const mcpServer = new McpServer({
    name: 'tools-with-sample-server',
    version: '1.0.0'
});

// Tool that uses LLM sampling to summarize any text
mcpServer.registerTool(
    'summarize',
    {
        description: 'Summarize any text using an LLM',
        inputSchema: z.object({
            text: z.string().describe('Text to summarize')
        })
    },
    async ({ text }) => {
        // Call the LLM through MCP sampling
        const response = await mcpServer.server.createMessage({
            messages: [
                {
                    role: 'user',
                    content: {
                        type: 'text',
                        text: `Please summarize the following text concisely:\n\n${text}`
                    }
                }
            ],
            maxTokens: 500
        });

        // Since we're not passing tools param to createMessage, response.content is single content
        return {
            content: [
                {
                    type: 'text',
                    text: response.content.type === 'text' ? response.content.text : 'Unable to generate summary'
                }
            ]
        };
    }
);

async function main() {
    const transport = new StdioServerTransport();
    await mcpServer.connect(transport);
    console.log('MCP server is running...');
}

try {
    await main();
} catch (error) {
    console.error('Server error:', error);
    // eslint-disable-next-line unicorn/no-process-exit
    process.exit(1);
}
