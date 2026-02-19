// T-APP-001.5: App lifecycle — MCP ext-apps integration
import { App } from "@modelcontextprotocol/ext-apps";
import { applyTheme } from "./theme";
import { createGraph } from "./graph";
import type { BuildContextResult } from "./types";

const app = new App({ name: "Concept Map Explorer", version: "1.0.0" }, {});
const graph = createGraph(document.getElementById("app")!);

function firstTextContent(content: { type: string; [key: string]: unknown }[]): string | null {
  const item = content?.find((c) => c.type === "text");
  if (!item) return null;
  return (item as { type: "text"; text: string }).text;
}

// T-APP-001.8: Error handling — prevent unhandled promise rejection on node click
graph.onNodeClick = async (conceptName: string) => {
  try {
    const result = await app.callServerTool({
      name: "BuildContext",
      arguments: { conceptName, depth: 1, maxEntities: 15, includeContent: false }
    });
    const text = firstTextContent(result.content ?? []);
    if (text) {
      const data: BuildContextResult = JSON.parse(text);
      graph.mergeData(data);
    }
  } catch {
    // Failed to expand concept — silently ignore
  }
};

app.ontoolresult = (result) => {
  const text = firstTextContent(result.content ?? []);
  if (text) {
    try {
      const data: BuildContextResult = JSON.parse(text);
      graph.mergeData(data);
    } catch { /* non-JSON result, ignore */ }
  }
};

app.onhostcontextchanged = (ctx) => applyTheme(ctx);
app.onteardown = async () => ({ state: {} });

await app.connect();
