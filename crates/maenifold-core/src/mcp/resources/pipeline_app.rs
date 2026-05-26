use crate::error::{MaenifoldError, Result};
use crate::mcp::tool_utils::{MCP_APP_HTML_MIME_TYPE, PIPELINE_UI_RESOURCE_URI};
use rmcp::model::{AnnotateAble, RawResource, ReadResourceResult, Resource, ResourceContents};

const PIPELINE_UI_MIME_TYPE: &str = MCP_APP_HTML_MIME_TYPE;

pub(super) fn list_resources() -> Vec<Resource> {
    vec![
        RawResource::new(PIPELINE_UI_RESOURCE_URI, "Pipeline MCP App")
            .with_title("Pipeline")
            .with_description("Interactive MCP App for inspecting Pipeline tool view results.")
            .with_mime_type(PIPELINE_UI_MIME_TYPE)
            .optional_annotate(None),
    ]
}

pub(super) fn is_pipeline_app_uri(uri: &str) -> bool {
    uri == PIPELINE_UI_RESOURCE_URI
}

pub(super) fn handle_read_resource(uri: &str) -> Result<ReadResourceResult> {
    if !is_pipeline_app_uri(uri) {
        return Err(MaenifoldError::InvalidResourceUri(format!(
            "{uri}. Expected '{PIPELINE_UI_RESOURCE_URI}'"
        )));
    }
    Ok(ReadResourceResult::new(vec![
        ResourceContents::text(HTML, uri.to_string()).with_mime_type(PIPELINE_UI_MIME_TYPE),
    ]))
}

pub(super) const HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Pipeline</title>
  <style>
    :root {
      color-scheme: light dark;
      --bg: Canvas;
      --fg: CanvasText;
      --muted: color-mix(in srgb, CanvasText 62%, Canvas 38%);
      --line: color-mix(in srgb, CanvasText 16%, Canvas 84%);
      --surface: color-mix(in srgb, Canvas 92%, CanvasText 8%);
      --ready: #2563eb;
      --active: #b45309;
      --done: #15803d;
      --idle: #64748b;
      --error: #b91c1c;
    }
    * { box-sizing: border-box; }
    [hidden] { display: none !important; }
    body {
      margin: 0;
      font: 14px/1.45 ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      background: var(--bg);
      color: var(--fg);
    }
    main {
      display: grid;
      grid-template-rows: auto auto 1fr;
      min-height: 100vh;
    }
    header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 16px;
      padding: 16px 20px 12px;
      border-bottom: 1px solid var(--line);
    }
    h1 {
      margin: 0;
      font-size: 20px;
      font-weight: 650;
    }
    .status {
      display: inline-flex;
      align-items: center;
      min-height: 28px;
      padding: 4px 10px;
      border: 1px solid var(--line);
      border-radius: 6px;
      font-size: 13px;
      color: var(--muted);
      background: var(--surface);
      white-space: nowrap;
    }
    .stats {
      display: grid;
      grid-template-columns: repeat(4, minmax(0, 1fr));
      gap: 1px;
      border-bottom: 1px solid var(--line);
      background: var(--line);
    }
    .stat {
      min-width: 0;
      padding: 12px 20px;
      background: var(--bg);
    }
    .stat b {
      display: block;
      font-size: 22px;
      line-height: 1.1;
    }
    .stat span {
      color: var(--muted);
      font-size: 12px;
      text-transform: uppercase;
    }
    .content {
      display: grid;
      grid-template-columns: minmax(0, 1fr) 320px;
      min-height: 0;
    }
    .nodes {
      min-width: 0;
      padding: 16px;
      overflow: auto;
    }
    .node-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
      gap: 12px;
    }
    .node {
      min-width: 0;
      border: 1px solid var(--line);
      border-radius: 8px;
      padding: 12px;
      background: var(--surface);
    }
    .node-head {
      display: flex;
      justify-content: space-between;
      align-items: flex-start;
      gap: 8px;
      min-width: 0;
    }
    .node-title {
      min-width: 0;
      overflow-wrap: anywhere;
      font-weight: 650;
    }
    .chip {
      flex: none;
      border-radius: 999px;
      padding: 2px 8px;
      font-size: 12px;
      color: white;
      background: var(--idle);
    }
    .chip.ready { background: var(--ready); }
    .chip.active { background: var(--active); }
    .chip.completed { background: var(--done); }
    .chip.failed, .chip.cancelled { background: var(--error); }
    dl {
      display: grid;
      grid-template-columns: 84px minmax(0, 1fr);
      gap: 6px 8px;
      margin: 10px 0 0;
      color: var(--muted);
    }
    dt { font-size: 12px; }
    dd {
      margin: 0;
      min-width: 0;
      overflow-wrap: anywhere;
      color: var(--fg);
    }
    aside {
      min-width: 0;
      border-left: 1px solid var(--line);
      overflow: auto;
    }
    .node[role="button"] { cursor: pointer; }
    .node[aria-selected="true"] {
      outline: 2px solid var(--ready);
      outline-offset: 2px;
    }
    section {
      padding: 16px;
      border-bottom: 1px solid var(--line);
    }
    h2 {
      margin: 0 0 10px;
      font-size: 13px;
      text-transform: uppercase;
      color: var(--muted);
    }
    ul {
      list-style: none;
      margin: 0;
      padding: 0;
    }
    li {
      padding: 6px 0;
      border-top: 1px solid var(--line);
      overflow-wrap: anywhere;
    }
    li:first-child { border-top: 0; }
    .empty {
      display: grid;
      place-items: center;
      min-height: 320px;
      color: var(--muted);
      text-align: center;
    }
    .detail {
      color: var(--muted);
      overflow-wrap: anywhere;
    }
    .detail pre {
      margin: 8px 0 0;
      white-space: pre-wrap;
      overflow-wrap: anywhere;
      font: 12px/1.4 ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
      color: var(--fg);
    }
    @media (max-width: 760px) {
      header { align-items: flex-start; flex-direction: column; }
      .stats { grid-template-columns: repeat(2, minmax(0, 1fr)); }
      .content { grid-template-columns: 1fr; }
      aside { border-left: 0; border-top: 1px solid var(--line); }
    }
  </style>
</head>
<body>
  <main>
    <header>
      <h1 id="title">Pipeline</h1>
      <div class="status" id="pipeline-status">No pipeline view loaded</div>
    </header>
    <div class="stats" aria-label="Pipeline state counts">
      <div class="stat"><b id="ready-count">0</b><span>Ready</span></div>
      <div class="stat"><b id="active-count">0</b><span>Active</span></div>
      <div class="stat"><b id="completed-count">0</b><span>Completed</span></div>
      <div class="stat"><b id="running-count">0</b><span>Running</span></div>
    </div>
    <div class="content">
      <div class="nodes">
        <div class="empty" id="empty">No pipeline nodes available.</div>
        <div class="node-grid" id="node-grid" hidden></div>
      </div>
      <aside>
        <section>
          <h2>Selected node</h2>
          <div class="detail" id="selected-node">Select a node.</div>
        </section>
        <section>
          <h2>Ready</h2>
          <ul id="ready-list"></ul>
        </section>
        <section>
          <h2>Active</h2>
          <ul id="active-list"></ul>
        </section>
        <section>
          <h2>Edges</h2>
          <ul id="edge-list"></ul>
        </section>
      </aside>
    </div>
  </main>
  <script type="application/json" id="pipeline-data"></script>
  <script>
    (() => {
      const byId = (id) => document.getElementById(id);
      const ids = (value) => {
        if (Array.isArray(value)) return value.map(itemId).filter(Boolean);
        if (value && typeof value === "object") return Object.keys(value);
        if (typeof value === "string" && value.trim()) return [value.trim()];
        return [];
      };
      const itemId = (item) => {
        if (typeof item === "string") return item;
        if (!item || typeof item !== "object") return "";
        return item.nodeId || item.id || item.name || "";
      };
      const nodeId = (node) => itemId(node) || "unnamed";
      const nodeKind = (node) => node.type || node.kind || node.nodeType || "node";
      const edgeLabel = (edge) => {
        if (!edge || typeof edge !== "object") return String(edge || "");
        const from = edge.from || edge.source || edge.sourceId || "?";
        const to = edge.to || edge.target || edge.targetId || "?";
        const mode = edge.type || edge.edgeType || edge.mode || "";
        return mode ? `${from} -> ${to} (${mode})` : `${from} -> ${to}`;
      };
      const statusClass = (value) => String(value || "idle").toLowerCase().replace(/[^a-z0-9_-]/g, "");
      const clear = (el) => { while (el.firstChild) el.removeChild(el.firstChild); };
      let selectedNodeId = "";
      const appendItems = (id, items) => {
        const list = byId(id);
        clear(list);
        for (const value of items) {
          const li = document.createElement("li");
          li.textContent = value;
          list.appendChild(li);
        }
      };
      const countRunning = (view, active) => {
        if (typeof view.running === "boolean") return view.running ? 1 : 0;
        if (Array.isArray(view.running)) return view.running.length;
        if (view.running && typeof view.running === "object") return Object.keys(view.running).length;
        return active.length;
      };
      const nodeStatus = (node, ready, active, completed) => {
        const id = nodeId(node);
        if (node.status) return node.status;
        if (completed.includes(id)) return "completed";
        if (active.includes(id)) return "active";
        if (ready.includes(id)) return "ready";
        return "idle";
      };
      const interestingNodeDetail = (node) => ({
        nodeId: node.nodeId || node.id,
        type: node.type,
        status: node.status,
        attempts: node.attempts,
        sessionId: node.sessionId,
        acpSessionId: node.acpSessionId,
        thinkingSessionId: node.thinkingSessionId,
        output: node.output,
        outputArtifact: node.outputArtifact,
        inputArtifact: node.inputArtifact,
        provenance: node.provenance,
        failureReason: node.failureReason
      });
      const compact = (value) => JSON.stringify(value, (key, item) => {
        if (item === null || item === "" || typeof item === "undefined") return undefined;
        return item;
      }, 2);
      const renderSelected = (node) => {
        const target = byId("selected-node");
        clear(target);
        if (!node) {
          target.textContent = "Select a node.";
          return;
        }
        const title = document.createElement("div");
        title.textContent = nodeId(node);
        const pre = document.createElement("pre");
        pre.textContent = compact(interestingNodeDetail(node)) || "{}";
        target.append(title, pre);
      };
      const renderNode = (node, ready, active, completed) => {
        const article = document.createElement("article");
        article.className = "node";
        article.setAttribute("role", "button");
        article.tabIndex = 0;
        article.setAttribute("aria-selected", String(nodeId(node) === selectedNodeId));
        article.addEventListener("click", () => {
          selectedNodeId = nodeId(node);
          render(window.__pipelineView);
        });
        article.addEventListener("keydown", (event) => {
          if (event.key === "Enter" || event.key === " ") {
            event.preventDefault();
            selectedNodeId = nodeId(node);
            render(window.__pipelineView);
          }
        });
        const head = document.createElement("div");
        head.className = "node-head";
        const title = document.createElement("div");
        title.className = "node-title";
        title.textContent = nodeId(node);
        const status = nodeStatus(node, ready, active, completed);
        const chip = document.createElement("span");
        chip.className = `chip ${statusClass(status)}`;
        chip.textContent = status;
        head.append(title, chip);
        const details = document.createElement("dl");
        for (const [label, value] of [
          ["Type", nodeKind(node)],
          ["Label", node.displayLabel || node.label || ""],
          ["Tool", node.toolName || node.tool_name || ""],
          ["Session", node.sessionId || node.session_id || ""]
        ]) {
          if (!value) continue;
          const dt = document.createElement("dt");
          dt.textContent = label;
          const dd = document.createElement("dd");
          dd.textContent = value;
          details.append(dt, dd);
        }
        article.append(head, details);
        return article;
      };
      const render = (view) => {
        if (!view || typeof view !== "object") return;
        window.__pipelineView = view;
        const nodes = Array.isArray(view.nodes) ? view.nodes : [];
        const edges = Array.isArray(view.edges) ? view.edges : [];
        const ready = ids(view.ready);
        const active = ids(view.active);
        const completed = ids(view.completed);
        const running = countRunning(view, active);
        byId("title").textContent = view.pipelineId || view.sessionId || view.id || "Pipeline";
        byId("pipeline-status").textContent = `Status: ${view.status || "unknown"}`;
        byId("ready-count").textContent = String(ready.length);
        byId("active-count").textContent = String(active.length);
        byId("completed-count").textContent = String(completed.length);
        byId("running-count").textContent = String(running);
        appendItems("ready-list", ready);
        appendItems("active-list", active);
        appendItems("edge-list", edges.map(edgeLabel));
        const grid = byId("node-grid");
        clear(grid);
        if (!selectedNodeId && nodes.length > 0) selectedNodeId = nodeId(nodes[0]);
        for (const node of nodes) grid.appendChild(renderNode(node, ready, active, completed));
        grid.hidden = nodes.length === 0;
        byId("empty").hidden = nodes.length !== 0;
        renderSelected(nodes.find((node) => nodeId(node) === selectedNodeId));
      };
      window.renderPipeline = render;
      window.setPipelineView = render;
      window.addEventListener("message", (event) => {
        const message = event.data || {};
        const params = message.params || {};
        const toolResult = params.result || params.toolResult || params;
        const payload = message.pipeline
          || message.structuredContent
          || params.structuredContent
          || toolResult.structuredContent
          || toolResult.pipeline
          || toolResult.data
          || message.data
          || message;
        render(payload);
      });
      const embedded = byId("pipeline-data").textContent.trim();
      if (embedded) {
        try { render(JSON.parse(embedded)); } catch (_) {}
      }
    })();
  </script>
</body>
</html>
"#;
