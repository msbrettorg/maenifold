import type { WebContents } from 'electron';
import { ipcMain } from 'electron';

type UserPromptResolvePayload =
  {
    kind: 'elicitation';
    id: string;
    response: {
      action: 'accept' | 'cancel' | 'dismiss';
      content?: Record<string, unknown>;
    };
  };

const USER_PROMPT_STREAM_URL = 'http://localhost:8080/api/user-prompt/stream';
const USER_PROMPT_RESPOND_URL = 'http://localhost:8080/api/user-prompt/respond';
const RECONNECT_MS = 2_000;

let activeSource: EventSource | null = null;
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;

function assertResolvePayload(value: unknown): UserPromptResolvePayload {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    throw new Error('invalid user prompt response');
  }
  const candidate = value as UserPromptResolvePayload;
  if (candidate.kind !== 'elicitation') {
    throw new Error('invalid user prompt kind');
  }
  if (typeof candidate.id !== 'string' || candidate.id.length === 0) {
    throw new Error('invalid user prompt id');
  }
  if (
    typeof candidate.response !== 'object'
    || candidate.response === null
    || Array.isArray(candidate.response)
  ) {
    throw new Error('invalid user prompt response');
  }
  const action = candidate.response?.action;
  if (action !== 'accept' && action !== 'cancel' && action !== 'dismiss') {
    throw new Error('invalid user prompt action');
  }
  if (
    candidate.response.content !== undefined
    && (typeof candidate.response.content !== 'object'
      || candidate.response.content === null
      || Array.isArray(candidate.response.content))
  ) {
    throw new Error('invalid user prompt content');
  }
  return candidate;
}

function scheduleReconnect(getWebContents: () => WebContents | null): void {
  if (reconnectTimer !== null) return;
  reconnectTimer = setTimeout(() => {
    reconnectTimer = null;
    connectUserPromptStream(getWebContents);
  }, RECONNECT_MS);
}

function connectUserPromptStream(getWebContents: () => WebContents | null): void {
  activeSource?.close();
  const EventSourceCtor = (globalThis as typeof globalThis & {
    EventSource?: new (url: string) => EventSource;
  }).EventSource;
  if (!EventSourceCtor) {
    scheduleReconnect(getWebContents);
    return;
  }
  const source = new EventSourceCtor(USER_PROMPT_STREAM_URL);
  activeSource = source;
  const forward = (event: MessageEvent) => {
    if (typeof event.data !== 'string') return;
    const webContents = getWebContents();
    if (webContents && !webContents.isDestroyed()) {
      webContents.send('mcp:user-prompt:request', JSON.parse(event.data));
    }
  };
  source.onmessage = forward;
  source.addEventListener('user-prompt', (event) => forward(event as MessageEvent));
  source.onerror = () => {
    if (activeSource === source) activeSource = null;
    source.close();
    scheduleReconnect(getWebContents);
  };
}

export function registerUserPromptBridge(getWebContents: () => WebContents | null): void {
  ipcMain.handle('mcp:user-prompt:resolve', async (_event, payload: unknown) => {
    const body = assertResolvePayload(payload);
    const response = await fetch(USER_PROMPT_RESPOND_URL, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    if (!response.ok) {
      throw new Error(`user prompt response failed with ${response.status}`);
    }
    return { ok: true };
  });
  connectUserPromptStream(getWebContents);
}
