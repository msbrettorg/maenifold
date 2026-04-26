import cors from "cors";
import { randomUUID } from "node:crypto";
import express from "express";
import type { Response } from "express";
import WebSocket, { type RawData } from "ws";

const MAENIFOLD_PORT = process.env.MAENIFOLD_PORT ?? "8080";
const AGENT_NAME = process.env.MAENIFOLD_AGENT ?? "Copilot";
const BRIDGE_PORT = Number(process.env.BRIDGE_PORT ?? "4000");
const ACP_TARGET = `ws://127.0.0.1:${MAENIFOLD_PORT}/api/acp/session?agent=${encodeURIComponent(AGENT_NAME)}`;
const SESSION_TTL_MS = 30 * 60 * 1000;

type JsonRpcMessage = {
  jsonrpc: "2.0";
  id?: number;
  method?: string;
  params?: Record<string, unknown>;
  result?: unknown;
  error?: { code: number; message: string; data?: unknown };
};

type PendingRequest = {
  resolve: (value: unknown) => void;
  reject: (reason: Error) => void;
  timer?: NodeJS.Timeout;
};

type RunContext = {
  res: Response;
  threadId: string;
  runId: string;
  closed: boolean;
  errored: boolean;
  textStarted: boolean;
  textEnded: boolean;
  messageId?: string;
  startedTools: Set<string>;
  endedTools: Set<string>;
};

type SessionEntry = {
  ws: WebSocket;
  sessionId: string;
  lastUsed: number;
  queue: Promise<void>;
  acp: AcpClient;
};

class AcpClient {
  private nextId = 1;
  private pending = new Map<number, PendingRequest>();
  activeRun?: RunContext;

  constructor(readonly ws: WebSocket) {
    ws.on("message", (data) => this.onMessage(data));
    ws.on("close", () => this.rejectAll(new Error("ACP WebSocket closed")));
    ws.on("error", (err) => this.rejectAll(err instanceof Error ? err : new Error(String(err))));
  }

  sendRequest(method: string, params: Record<string, unknown>, timeoutMs = 0): Promise<unknown> {
    const id = this.nextId++;
    const payload = { jsonrpc: "2.0", id, method, params };

    return new Promise((resolve, reject) => {
      const timer = timeoutMs > 0
        ? setTimeout(() => {
            this.pending.delete(id);
            reject(new Error(`ACP request timed out: ${method}`));
          }, timeoutMs)
        : undefined;

      this.pending.set(id, { resolve, reject, timer });
      console.log("[bridge] ACP ->", method, params);
      this.ws.send(JSON.stringify(payload), (err) => {
        if (!err) return;
        if (timer) clearTimeout(timer);
        this.pending.delete(id);
        reject(err);
      });
    });
  }

  private onMessage(data: RawData) {
    let message: JsonRpcMessage;
    try {
      message = JSON.parse(data.toString()) as JsonRpcMessage;
    } catch (err) {
      console.error("[bridge] Failed to parse ACP message", err);
      return;
    }

    console.log("[bridge] ACP <-", JSON.stringify(message));

    if (typeof message.id === "number") {
      const pending = this.pending.get(message.id);
      if (!pending) return;
      this.pending.delete(message.id);
      if (pending.timer) clearTimeout(pending.timer);
      if (message.error) {
        pending.reject(new Error(message.error.message));
      } else {
        pending.resolve(message.result);
      }
      return;
    }

    if (message.method === "session/update" && this.activeRun) {
      handleSessionUpdate(this.activeRun, message.params ?? {});
    }
  }

  private rejectAll(error: Error) {
    for (const [id, pending] of this.pending) {
      if (pending.timer) clearTimeout(pending.timer);
      pending.reject(error);
      this.pending.delete(id);
    }
    if (this.activeRun && !this.activeRun.closed) {
      this.activeRun.errored = true;
      sendEvent(this.activeRun.res, { type: "RUN_ERROR", threadId: this.activeRun.threadId, runId: this.activeRun.runId, message: error.message });
      this.activeRun.res.end();
      this.activeRun.closed = true;
    }
  }
}

const sessions = new Map<string, SessionEntry>();

const app = express();
app.use(cors({ origin: true }));
app.use(express.json({ limit: "2mb" }));

app.get("/info", (_req, res) => {
  res.json({ name: "maenifold-copilotkit-bridge", version: "0.1.0" });
});

app.post("/", async (req, res) => {
  const raw = req.body ?? {};
  // CopilotKit may nest under params or send flat — normalize
  const body = (raw.params && typeof raw.params === "object") ? raw.params as Record<string, unknown> : raw;
  const threadId = String(body.threadId ?? body.thread_id ?? randomUUID());
  const runId = String(body.runId ?? body.run_id ?? randomUUID());
  const promptText = extractLatestUserText(body);
  console.log("[bridge] Incoming request", { method: raw.method, threadId, promptText: promptText.slice(0, 80) });

  prepareSse(res);

  const ctx: RunContext = {
    res,
    threadId,
    runId,
    closed: false,
    errored: false,
    textStarted: false,
    textEnded: false,
    startedTools: new Set(),
    endedTools: new Set(),
  };

  req.on("aborted", () => {
    ctx.closed = true;
  });

  res.on("close", () => {
    if (!res.writableEnded) {
      ctx.closed = true;
    }
  });

  sendEvent(res, { type: "RUN_STARTED", threadId, runId });

  try {
    console.log("[bridge] Getting session for", threadId);
    const session = await getSession(threadId);
    console.log("[bridge] Session obtained:", session.sessionId, "starting prompt loop");
    await runPromptLoop(session, ctx, promptText);
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    console.error("[bridge]", message);
    ctx.errored = true;
    if (!ctx.closed) {
      sendEvent(res, { type: "RUN_ERROR", threadId, runId, message });
    }
  } finally {
    if (!ctx.errored) {
      finishTextIfNeeded(ctx);
    }
    if (!ctx.closed) {
      if (!ctx.errored) {
        sendEvent(res, { type: "RUN_FINISHED", threadId, runId });
      }
      res.end();
      ctx.closed = true;
    }
  }
});

async function getSession(threadId: string): Promise<SessionEntry> {
  cleanupStaleSessions();

  const existing = sessions.get(threadId);
  if (existing && existing.ws.readyState === WebSocket.OPEN) {
    existing.lastUsed = Date.now();
    return existing;
  }

  if (existing) {
    existing.ws.close();
    sessions.delete(threadId);
  }

  const ws = await openWebSocket(ACP_TARGET);
  const acp = new AcpClient(ws);

  await acp.sendRequest("initialize", {
    protocolVersion: 1,
    clientInfo: { name: "copilotkit-bridge", version: "0.1.0" },
    clientCapabilities: {},
  }, 30_000);

  const sessionNew = await acp.sendRequest("session/new", {}, 30_000) as { sessionId?: string; id?: string };
  const sessionId = sessionNew.sessionId ?? sessionNew.id;
  if (!sessionId) {
    throw new Error("ACP session/new response did not include sessionId");
  }

  const entry: SessionEntry = { ws, sessionId, lastUsed: Date.now(), queue: Promise.resolve(), acp };
  sessions.set(threadId, entry);
  return entry;
}

async function runPromptLoop(session: SessionEntry, ctx: RunContext, promptText: string) {
  session.lastUsed = Date.now();
  session.acp.activeRun = ctx;

  try {
    let content: Array<{ type: "text"; text: string }> = [{ type: "text", text: promptText }];

    while (!ctx.closed) {
      console.log("[bridge] ACP -> session/prompt", { sessionId: session.sessionId, contentLength: content.length, text: content[0]?.text?.slice(0, 80) });
      const result = await session.acp.sendRequest("session/prompt", {
        sessionId: session.sessionId,
        content,
      }) as { stopReason?: string };

      const stopReason = result.stopReason ?? "endTurn";
      console.log("[bridge] stopReason:", stopReason);

      if (stopReason === "toolUse") {
        finishTextIfNeeded(ctx);
        content = [];
        continue;
      }

      if (stopReason === "cancelled") {
        throw new Error("ACP prompt was cancelled");
      }

      break;
    }
  } finally {
    if (session.acp.activeRun === ctx) {
      session.acp.activeRun = undefined;
    }
  }
}

function handleSessionUpdate(ctx: RunContext, params: Record<string, unknown>) {
  const updateType = String(params.sessionUpdate ?? "");

  if (updateType === "agent_message_chunk") {
    const delta = String(params.delta ?? "");
    if (!delta) return;

    if (!ctx.textStarted || ctx.textEnded) {
      ctx.messageId = String(params.messageId ?? randomUUID());
      ctx.textStarted = true;
      ctx.textEnded = false;
      sendEvent(ctx.res, { type: "TEXT_MESSAGE_START", messageId: ctx.messageId, role: "assistant" });
    }

    sendEvent(ctx.res, { type: "TEXT_MESSAGE_CONTENT", messageId: ctx.messageId, delta });
    return;
  }

  if (updateType === "agent_thought_chunk") {
    const delta = String(params.delta ?? "");
    if (delta) {
      sendEvent(ctx.res, { type: "THOUGHT_MESSAGE_CONTENT", delta });
    }
    return;
  }

  if (updateType === "tool_call" || updateType === "tool_call_update") {
    finishTextIfNeeded(ctx);
    const toolCalls = Array.isArray(params.toolCalls) ? params.toolCalls : [];
    for (const rawToolCall of toolCalls) {
      const toolCall = rawToolCall as Record<string, unknown>;
      const toolCallId = String(toolCall.id ?? randomUUID());
      const toolCallName = String(toolCall.name ?? "tool");
      const status = String(toolCall.status ?? "");

      if (!ctx.startedTools.has(toolCallId) && (status === "pending" || status === "in_progress" || updateType === "tool_call")) {
        ctx.startedTools.add(toolCallId);
        sendEvent(ctx.res, { type: "TOOL_CALL_START", toolCallId, toolCallName, parentMessageId: ctx.messageId });
        if (toolCall.arguments !== undefined) {
          sendEvent(ctx.res, { type: "TOOL_CALL_ARGS", toolCallId, delta: stringifyToolValue(toolCall.arguments) });
        }
      }

      if (!ctx.endedTools.has(toolCallId) && (status === "completed" || status === "failed")) {
        ctx.endedTools.add(toolCallId);
        sendEvent(ctx.res, { type: "TOOL_CALL_END", toolCallId });
      }
    }
  }
}

function prepareSse(res: Response) {
  res.status(200);
  res.setHeader("Content-Type", "text/event-stream; charset=utf-8");
  res.setHeader("Cache-Control", "no-cache, no-transform");
  res.setHeader("Connection", "keep-alive");
  res.flushHeaders?.();
}

function sendEvent(res: Response, event: Record<string, unknown>) {
  res.write(`data: ${JSON.stringify(event)}\n\n`);
}

function finishTextIfNeeded(ctx: RunContext) {
  if (ctx.textStarted && !ctx.textEnded && ctx.messageId && !ctx.closed) {
    sendEvent(ctx.res, { type: "TEXT_MESSAGE_END", messageId: ctx.messageId });
    ctx.textEnded = true;
  }
}

function extractLatestUserText(body: Record<string, unknown>): string {
  const messages = Array.isArray(body.messages) ? body.messages : [];
  const latestUser = [...messages].reverse().find((message) => {
    const record = message as Record<string, unknown>;
    return record.role === "user" || record.role === "human";
  }) as Record<string, unknown> | undefined;

  const content = latestUser?.content ?? body.input ?? body.prompt ?? "";
  if (typeof content === "string") return content;
  if (Array.isArray(content)) {
    return content.map((part) => {
      if (typeof part === "string") return part;
      const record = part as Record<string, unknown>;
      return typeof record.text === "string" ? record.text : "";
    }).join("");
  }
  return String(content);
}

function stringifyToolValue(value: unknown): string {
  return typeof value === "string" ? value : JSON.stringify(value ?? {});
}

function openWebSocket(url: string): Promise<WebSocket> {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(url);
    const timer = setTimeout(() => {
      ws.close();
      reject(new Error(`Timed out connecting to ACP WebSocket: ${url}`));
    }, 30_000);

    ws.once("open", () => {
      clearTimeout(timer);
      console.log("[bridge] Connected to ACP", url);
      resolve(ws);
    });

    ws.once("error", (err) => {
      clearTimeout(timer);
      reject(err);
    });
  });
}

function cleanupStaleSessions() {
  const cutoff = Date.now() - SESSION_TTL_MS;
  for (const [threadId, session] of sessions) {
    if (session.lastUsed >= cutoff) continue;
    console.log("[bridge] Evicting stale session", threadId);
    session.ws.close();
    sessions.delete(threadId);
  }
}

app.listen(BRIDGE_PORT, "127.0.0.1", () => {
  console.log(`Bridge listening on http://127.0.0.1:${BRIDGE_PORT}`);
  console.log(`ACP target: ${ACP_TARGET}`);
});
