import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';

const RESOURCE_URI = 'ui://test-app/chart';

const appHtml = String.raw`<!doctype html>
<html>
  <head>
    <meta charset="utf-8">
    <meta name="color-scheme" content="light dark">
    <style>
      body {
        margin: 0;
        padding: 16px;
        font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
        background: canvas;
        color: canvastext;
      }
      button {
        border: 1px solid color-mix(in srgb, canvastext 25%, transparent);
        border-radius: 8px;
        padding: 8px 12px;
        background: color-mix(in srgb, canvastext 8%, transparent);
        color: inherit;
        cursor: pointer;
      }
      #result {
        margin-top: 12px;
        padding: 12px;
        border-radius: 8px;
        background: color-mix(in srgb, canvastext 6%, transparent);
        white-space: pre-wrap;
      }
    </style>
  </head>
  <body>
    <h1>MCP Apps callback fixture</h1>
    <p>This app verifies iframe tools/call proxying back to its originating MCP server.</p>
    <button id="refresh" type="button">Refresh Data</button>
    <div id="result">Waiting for data...</div>
    <script type="module" src="mcp-app-bridge.js"></script>
    <script type="module">
      import { App, PostMessageTransport } from '@modelcontextprotocol/ext-apps/app-with-deps';

      const result = document.getElementById('result');
      const refresh = document.getElementById('refresh');
      const app = new App(
        { name: 'mcp-apps-callback-fixture', version: '0.1.0' },
        {},
        { autoResize: true },
      );

      function render(value) {
        result.textContent = JSON.stringify(value, null, 2);
      }

      refresh.addEventListener('click', async () => {
        result.textContent = 'Loading...';
        try {
          const response = await app.callServerTool({
            name: 'get_data',
            arguments: {},
          });
          render(response.structuredContent ?? response.content ?? response);
        } catch (error) {
          result.textContent = error instanceof Error ? error.message : String(error);
        }
      });

      await app.connect(new PostMessageTransport(window.parent, window.parent));
    </script>
  </body>
</html>`;

const server = new McpServer({
  name: 'mcp-apps-test-server',
  version: '0.1.0',
});

server.registerTool(
  'render_chart',
  {
    title: 'Render chart fixture',
    description: 'Returns an MCP Apps UI resource for callback proxy E2E verification.',
    _meta: {
      ui: {
        resourceUri: RESOURCE_URI,
      },
    },
  },
  async () => ({
    content: [{ type: 'text', text: 'Interactive chart fixture rendered.' }],
    structuredContent: { rendered: true },
    _meta: {
      ui: {
        resourceUri: RESOURCE_URI,
      },
    },
  }),
);

server.registerTool(
  'get_data',
  {
    title: 'Get data fixture',
    description: 'Callback target for the MCP Apps iframe fixture.',
  },
  async () => ({
    content: [{ type: 'text', text: JSON.stringify({ items: ['data1', 'data2'] }) }],
    structuredContent: { items: ['data1', 'data2'] },
    _meta: { source: 'mcp-apps-test-server' },
  }),
);

server.registerResource(
  'test-app-chart',
  RESOURCE_URI,
  {
    title: 'MCP Apps chart fixture',
    description: 'HTML UI that calls get_data through the host tools/call proxy.',
    mimeType: 'text/html',
  },
  async () => ({
    contents: [
      {
        uri: RESOURCE_URI,
        mimeType: 'text/html',
        text: appHtml,
      },
    ],
  }),
);

const transport = new StdioServerTransport();
await server.connect(transport);
