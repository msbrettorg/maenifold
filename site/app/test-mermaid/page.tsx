// T-SITE-001.0: RTM NFR-15.6 — build-time Mermaid rendering validation page
// Server component: renderMermaid is called at build time; no client JS is emitted.

import { renderMermaid } from "@/lib/mermaid";

const TEST_DIAGRAM = `graph TD
  A[WikiLinks] --> B[Graph]
  B --> C[Search]
  C --> D[Session]
  D --> E[Reasoning]
  E --> F[Orchestration]`;

export default async function TestMermaidPage() {
  const svg = await renderMermaid(TEST_DIAGRAM);

  return (
    <main
      style={{
        background: "#1E1B22",
        minHeight: "100vh",
        padding: "2rem",
        fontFamily: "monospace",
        color: "#E8E0DB",
      }}
    >
      <h1 style={{ color: "#6AABB3", marginBottom: "1rem" }}>
        T-SITE-001.0: Mermaid Build-time Rendering Spike
      </h1>
      <p style={{ marginBottom: "2rem", opacity: 0.7 }}>
        The diagram below was pre-rendered at build time via mmdc. No Mermaid
        JavaScript is shipped to the client.
      </p>
      {/* SVG rendered at build time — no client Mermaid JS */}
      <div
        style={{
          background: "#2A3A3D",
          borderRadius: "8px",
          padding: "1.5rem",
          border: "1px solid #6AABB3",
          display: "inline-block",
        }}
        // biome-ignore lint/security/noDangerouslySetInnerHtml: intentional build-time SVG
        dangerouslySetInnerHTML={{ __html: svg }}
      />
    </main>
  );
}
