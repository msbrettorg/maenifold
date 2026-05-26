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