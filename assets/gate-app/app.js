    (() => {
      const STAGES = ["pending", "swe", "red-team", "blue-team", "done"];
      const PRIORITY_ORDER = { p0: 0, p1: 1, p2: 2, p3: 3 };
      const byId = (id) => document.getElementById(id);
      const clear = (el) => { while (el.firstChild) el.removeChild(el.firstChild); };
      const taskState = (task) => String(task.state || task.stage || "pending").toLowerCase();
      const taskId = (task) => String(task.taskId || task.id || "unknown");
      const priorityText = (task) => String(task.priority || "P2").toUpperCase();
      const priorityRank = (task) => PRIORITY_ORDER[priorityText(task).toLowerCase()] ?? 99;
      const isStale = (task) => {
        const stamp = task.lastUpdatedAt;
        if (!stamp) return false;
        const millis = Date.parse(stamp);
        return Number.isFinite(millis) && (Date.now() - millis) > 24 * 60 * 60 * 1000;
      };
      const compact = (value) => JSON.stringify(value, (k, item) =>
        item === null || item === "" || typeof item === "undefined" ? undefined : item, 2
      );
      const parsePayload = (value) => {
        if (Array.isArray(value)) return value;
        if (value && Array.isArray(value.tasks)) return value.tasks;
        if (value && value.data && Array.isArray(value.data.tasks)) return value.data.tasks;
        return [];
      };
      let selectedTaskId = "";
      let lastTasks = [];
      const renderInspector = () => {
        const task = lastTasks.find((item) => taskId(item) === selectedTaskId);
        const selected = byId("selected-task");
        const record = byId("selected-record");
        clear(selected);
        clear(record);
        if (!task) {
          selected.textContent = "Select a task card.";
          record.textContent = "No record selected.";
          return;
        }
        const pre = document.createElement("pre");
        pre.textContent = compact({
          taskId: task.taskId, title: task.title, state: task.state, pipeline: task.pipeline,
          priority: task.priority, sprint: task.sprint, blocked: task.blocked,
          blocked_reason: task.blocked_reason, lastUpdatedAt: task.lastUpdatedAt,
          location: task.location, path: task.path
        }) || "{}";
        selected.appendChild(pre);
        const recordPre = document.createElement("pre");
        recordPre.textContent = compact(task.latestRecord || task.lastRecord || null) || "null";
        record.appendChild(recordPre);
      };
      const renderCard = (task) => {
        const card = document.createElement("article");
        card.className = "card";
        card.tabIndex = 0;
        card.setAttribute("role", "button");
        card.setAttribute("aria-selected", String(taskId(task) === selectedTaskId));
        card.addEventListener("click", () => { selectedTaskId = taskId(task); renderColumns(lastTasks); });
        card.addEventListener("keydown", (event) => {
          if (event.key === "Enter" || event.key === " ") {
            event.preventDefault();
            selectedTaskId = taskId(task);
            renderColumns(lastTasks);
          }
        });
        const head = document.createElement("div");
        head.className = "card-head";
        const id = document.createElement("div");
        id.className = "task-id";
        id.textContent = taskId(task);
        head.appendChild(id);
        const title = document.createElement("div");
        title.className = "task-title";
        title.textContent = task.title || "(untitled)";
        const chipRow = document.createElement("div");
        chipRow.className = "chip-row";
        const p = document.createElement("span");
        const pText = priorityText(task);
        p.className = `chip ${`priority-${pText.toLowerCase()}`}`;
        p.textContent = pText;
        chipRow.appendChild(p);
        const pipeline = document.createElement("span");
        pipeline.className = "chip pipeline";
        pipeline.textContent = String(task.pipeline || "SDD").toUpperCase();
        chipRow.appendChild(pipeline);
        if (isStale(task)) {
          const stale = document.createElement("span");
          stale.className = "chip stale";
          stale.textContent = "STALE";
          chipRow.appendChild(stale);
        }
        card.append(head, title, chipRow);
        return card;
      };
      const renderColumns = (tasks) => {
        const grouped = new Map(STAGES.map((stage) => [stage, []]));
        for (const task of tasks) {
          const stage = taskState(task);
          (grouped.get(stage) || grouped.get("pending")).push(task);
        }
        const grid = byId("kanban-grid");
        grid.style.setProperty("--stage-count", String(STAGES.length));
        clear(grid);
        for (const stage of STAGES) {
          const section = document.createElement("section");
          section.className = "column";
          const title = document.createElement("h2");
          title.textContent = stage;
          const cards = document.createElement("div");
          cards.className = "cards";
          const sorted = grouped.get(stage).sort((left, right) => {
            const byPriority = priorityRank(left) - priorityRank(right);
            return byPriority || taskId(left).localeCompare(taskId(right));
          });
          for (const task of sorted) cards.appendChild(renderCard(task));
          section.append(title, cards);
          grid.appendChild(section);
        }
        renderInspector();
      };
      const render = (payload) => {
        const tasks = parsePayload(payload);
        lastTasks = tasks;
        if (!selectedTaskId && tasks.length > 0) selectedTaskId = taskId(tasks[0]);
        byId("title").textContent = "Gate Board";
        byId("gate-status").textContent = `Loaded ${tasks.length} task${tasks.length === 1 ? "" : "s"}`;
        byId("task-count").textContent = String(tasks.length);
        byId("stage-count").textContent = String(STAGES.length);
        byId("stale-count").textContent = String(tasks.filter(isStale).length);
        byId("empty").hidden = tasks.length !== 0;
        byId("kanban-grid").hidden = tasks.length === 0;
        renderColumns(tasks);
      };
      window.renderGate = render;
      window.setGateView = render;
      window.addEventListener("message", (event) => {
        const message = event.data || {};
        const params = message.params || {};
        const toolResult = params.result || params.toolResult || params;
        const payload = message.structuredContent
          || params.structuredContent
          || toolResult.structuredContent
          || toolResult.data
          || message.data
          || message;
        render(payload);
      });
      const embedded = byId("gate-data").textContent.trim();
      if (embedded) {
        try { render(JSON.parse(embedded)); } catch (_) {}
      }
    })();
