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
      const escapeHtml = (value) => String(value ?? "")
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;")
        .replace(/'/g, "&#39;");
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
      let editor = null;
      let drawflowIdToPayloadId = new Map();
      let payloadNodeMap = new Map();
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
      const edgeFromId = (edge) => edge?.from || edge?.source || edge?.sourceId || "";
      const edgeToId = (edge) => edge?.to || edge?.target || edge?.targetId || "";
      const viewportSize = (mount) => {
        if (!mount) return { width: 0, height: 0 };
        const rect = mount.getBoundingClientRect();
        return {
          width: mount.clientWidth || rect.width || 0,
          height: mount.clientHeight || rect.height || 0
        };
      };
      const clamp = (value, min, max) => Math.max(min, Math.min(max, value));
      const layoutGraph = (nodes, edges, mount) => {
        const order = new Map();
        const incoming = new Map();
        const outgoing = new Map();
        const indegree = new Map();
        const depth = new Map();
        for (let index = 0; index < nodes.length; index += 1) {
          const id = nodeId(nodes[index]);
          order.set(id, index);
          incoming.set(id, []);
          outgoing.set(id, []);
          indegree.set(id, 0);
          depth.set(id, 0);
        }
        for (const edge of edges) {
          const from = edgeFromId(edge);
          const to = edgeToId(edge);
          if (!order.has(from) || !order.has(to)) continue;
          outgoing.get(from).push(to);
          incoming.get(to).push(from);
          indegree.set(to, indegree.get(to) + 1);
        }
        const queue = [];
        for (const node of nodes) {
          const id = nodeId(node);
          if (indegree.get(id) === 0) queue.push(id);
        }
        queue.sort((left, right) => order.get(left) - order.get(right));
        const processed = new Set();
        while (queue.length > 0) {
          const current = queue.shift();
          processed.add(current);
          for (const next of outgoing.get(current)) {
            const nextDepth = Math.max(depth.get(next), depth.get(current) + 1);
            depth.set(next, nextDepth);
            indegree.set(next, indegree.get(next) - 1);
            if (indegree.get(next) === 0) {
              queue.push(next);
              queue.sort((left, right) => order.get(left) - order.get(right));
            }
          }
        }
        for (const node of nodes) {
          const id = nodeId(node);
          if (processed.has(id)) continue;
          const inferredDepth = incoming.get(id).reduce(
            (maxDepth, upstreamId) => Math.max(maxDepth, (depth.get(upstreamId) || 0) + 1),
            0
          );
          depth.set(id, inferredDepth);
        }
        const columns = new Map();
        for (const node of nodes) {
          const id = nodeId(node);
          const column = depth.get(id) || 0;
          if (!columns.has(column)) columns.set(column, []);
          columns.get(column).push(id);
        }
        for (const columnNodes of columns.values()) {
          columnNodes.sort((left, right) => order.get(left) - order.get(right));
        }
        const maxColumn = columns.size > 0 ? Math.max(...columns.keys()) : 0;
        const maxRows = [...columns.values()].map((columnNodes) => columnNodes.length);
        const tallestColumn = maxRows.length > 0 ? Math.max(...maxRows) : 1;
        const viewport = viewportSize(mount);
        const usableWidth = Math.max(120, viewport.width - 80);
        const usableHeight = Math.max(120, viewport.height - 80);
        const xSpacing = Math.min(240, usableWidth / Math.max(1, maxColumn + 1));
        const ySpacing = Math.min(100, usableHeight / Math.max(1, tallestColumn));
        const positions = new Map();
        for (const [column, columnNodes] of columns.entries()) {
          columnNodes.forEach((id, row) => {
            positions.set(id, { column, row, x: column * xSpacing, y: row * ySpacing });
          });
        }
        return { positions };
      };
      const fitDAGToViewport = (mount, positions) => {
        if (!editor || !mount || !positions || positions.size === 0) return;
        const nodeWidth = 180;
        const nodeHeight = 64;
        const padding = 24;
        let minX = Number.POSITIVE_INFINITY;
        let minY = Number.POSITIVE_INFINITY;
        let maxX = Number.NEGATIVE_INFINITY;
        let maxY = Number.NEGATIVE_INFINITY;
        for (const position of positions.values()) {
          minX = Math.min(minX, position.x);
          minY = Math.min(minY, position.y);
          maxX = Math.max(maxX, position.x);
          maxY = Math.max(maxY, position.y);
        }
        if (!Number.isFinite(minX) || !Number.isFinite(minY)) return;
        const bboxWidth = Math.max(nodeWidth, (maxX - minX) + nodeWidth);
        const bboxHeight = Math.max(nodeHeight, (maxY - minY) + nodeHeight);
        const viewport = viewportSize(mount);
        if (viewport.width <= 0 || viewport.height <= 0) return;
        const fitWidth = Math.max(1, viewport.width - (padding * 2));
        const fitHeight = Math.max(1, viewport.height - (padding * 2));
        const rawZoom = Math.min(fitWidth / bboxWidth, fitHeight / bboxHeight) * 0.9;
        const zoomMin = typeof editor.zoom_min === "number" ? editor.zoom_min : 0.5;
        const zoomMax = typeof editor.zoom_max === "number" ? editor.zoom_max : 1.6;
        const zoom = clamp(Number.isFinite(rawZoom) ? rawZoom : 1, zoomMin, zoomMax);
        const centerX = minX + (bboxWidth / 2);
        const centerY = minY + (bboxHeight / 2);
        const targetCanvasX = (viewport.width / 2) - (centerX * zoom);
        const targetCanvasY = (viewport.height / 2) - (centerY * zoom);
        editor.zoom = zoom;
        editor.canvas_x = targetCanvasX;
        editor.canvas_y = targetCanvasY;
        if (typeof editor.zoom_refresh === "function") {
          const epsilon = 0.001;
          const bumpedZoom = clamp(
            zoom >= zoomMax ? zoom - epsilon : zoom + epsilon,
            zoomMin,
            zoomMax
          );
          if (Math.abs(bumpedZoom - zoom) > Number.EPSILON) {
            editor.zoom_last_value = bumpedZoom;
            editor.zoom_refresh();
          }
          editor.canvas_x = targetCanvasX;
          editor.canvas_y = targetCanvasY;
          editor.zoom_last_value = zoom;
          editor.zoom_refresh();
        }
      };
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
      const renderDAG = (nodes, edges, ready, active, completed) => {
        const mount = byId("drawflow");
        if (!editor || !mount) return;
        editor.clear();
        drawflowIdToPayloadId = new Map();
        payloadNodeMap = new Map(nodes.map((node) => [nodeId(node), node]));
        const { positions } = layoutGraph(nodes, edges, mount);
        const drawflowByPayloadId = new Map();
        for (const node of nodes) {
          const id = nodeId(node);
          const position = positions.get(id) || { column: 0, row: 0, x: 0, y: 0 };
          const status = nodeStatus(node, ready, active, completed);
          const htmlBody = `<div class="node-title">${escapeHtml(id)}</div><div class="node-type">${escapeHtml(nodeKind(node))}</div>`;
          const drawflowNodeId = editor.addNode(
            id,
            1,
            1,
            position.x,
            position.y,
            `node-${statusClass(status)}`,
            {},
            htmlBody
          );
          drawflowByPayloadId.set(id, drawflowNodeId);
          drawflowIdToPayloadId.set(String(drawflowNodeId), id);
        }
        for (const edge of edges) {
          const from = edgeFromId(edge);
          const to = edgeToId(edge);
          const fromDrawflowId = drawflowByPayloadId.get(from);
          const toDrawflowId = drawflowByPayloadId.get(to);
          if (!fromDrawflowId || !toDrawflowId) continue;
          editor.addConnection(fromDrawflowId, toDrawflowId, "output_1", "input_1");
        }
        fitDAGToViewport(mount, positions);
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
        if (!selectedNodeId && nodes.length > 0) selectedNodeId = nodeId(nodes[0]);
        const drawflow = byId("drawflow");
        const empty = byId("empty");
        drawflow.hidden = false;
        empty.hidden = true;
        renderDAG(nodes, edges, ready, active, completed);
        if (nodes.length === 0) {
          drawflow.hidden = true;
          empty.hidden = false;
        }
        renderSelected(nodes.find((node) => nodeId(node) === selectedNodeId));
      };
      window.renderPipeline = render;
      window.setPipelineView = render;
      document.addEventListener("DOMContentLoaded", () => {
        const mount = byId("drawflow");
        if (!mount || typeof Drawflow !== "function") return;
        editor = new Drawflow(mount);
        editor.reroute = true;
        editor.start();
        editor.on("nodeSelected", (id) => {
          const payloadId = drawflowIdToPayloadId.get(String(id));
          if (!payloadId) return;
          selectedNodeId = payloadId;
          renderSelected(payloadNodeMap.get(payloadId));
        });
        if (window.__pipelineView) render(window.__pipelineView);
      });
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