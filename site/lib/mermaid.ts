// T-SITE-001.0: RTM NFR-15.6 — build-time Mermaid rendering via mmdc
// Mermaid diagrams are pre-rendered to inline SVG at build time.
// No Mermaid JavaScript is shipped to the client.

import { execFile } from "child_process";
import { mkdtemp, readFile, rm, writeFile } from "fs/promises";
import { tmpdir } from "os";
import { join } from "path";
import { promisify } from "util";

const execFileAsync = promisify(execFile);

// Design system palette per NFR-15.6
const MERMAID_CONFIG = {
  theme: "base",
  themeVariables: {
    primaryColor: "#2A3A3D",
    primaryTextColor: "#E8E0DB",
    primaryBorderColor: "#6AABB3",
    lineColor: "#6AABB3",
    secondaryColor: "#1E1B22",
    tertiaryColor: "#2A2630",
    background: "#1E1B22",
    mainBkg: "#2A3A3D",
    nodeBorder: "#6AABB3",
    clusterBkg: "#1E1B22",
    clusterBorder: "#2A2630",
    titleColor: "#E8E0DB",
    edgeLabelBackground: "#1E1B22",
    nodeTextColor: "#E8E0DB",
  },
};

/**
 * Renders a Mermaid diagram code string to an inline SVG string at build time.
 *
 * Uses @mermaid-js/mermaid-cli (mmdc) as a subprocess. No Mermaid JS is
 * shipped to the client; all rendering happens during `next build`.
 *
 * @param code - The Mermaid diagram source (e.g. "graph TD\n  A --> B")
 * @returns The SVG markup as a string, suitable for dangerouslySetInnerHTML
 */
export async function renderMermaid(code: string): Promise<string> {
  const tmpDir = await mkdtemp(join(tmpdir(), "mermaid-"));
  const inputFile = join(tmpDir, "diagram.mmd");
  const outputFile = join(tmpDir, "diagram.svg");
  const configFile = join(tmpDir, "config.json");

  try {
    await writeFile(inputFile, code, "utf8");
    await writeFile(configFile, JSON.stringify(MERMAID_CONFIG), "utf8");

    // Resolve mmdc from the site's local node_modules to avoid PATH dependency.
    // __dirname is not available in ESM; use process.cwd() which is the project root
    // during `next build`. We walk up from cwd to find node_modules/.bin/mmdc.
    const mmdcPath = join(process.cwd(), "node_modules", ".bin", "mmdc");

    const startMs = Date.now();
    await execFileAsync(mmdcPath, [
      "--input", inputFile,
      "--output", outputFile,
      "--configFile", configFile,
      "--backgroundColor", "transparent",
    ], {
      // mmdc spawns a headless Chromium — allow enough time
      timeout: 30_000,
    });
    const elapsedMs = Date.now() - startMs;
    console.log(`[mermaid] rendered in ${elapsedMs}ms`);

    const svg = await readFile(outputFile, "utf8");
    return svg;
  } finally {
    await rm(tmpDir, { recursive: true, force: true });
  }
}
