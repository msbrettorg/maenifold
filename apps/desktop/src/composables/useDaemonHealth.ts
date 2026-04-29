import { ref } from 'vue';
import { getApiBase } from '../lib/api-base';

const ONLINE_INTERVAL_MS = 5000;
const OFFLINE_INTERVAL_MS = 2000;
const HEALTH_TIMEOUT_MS = 3000;

const daemonOnline = ref(true);
const lastCheckedAt = ref<number | null>(null);
const lastErrorMessage = ref<string | null>(null);

let pollTimer: ReturnType<typeof setTimeout> | null = null;
let running = false;
let inFlight: Promise<void> | null = null;
let activeController: AbortController | null = null;
let probeGeneration = 0;

function clearPollTimer(): void {
  if (pollTimer !== null) {
    clearTimeout(pollTimer);
    pollTimer = null;
  }
}

function getProbeInterval(): number {
  return daemonOnline.value ? ONLINE_INTERVAL_MS : OFFLINE_INTERVAL_MS;
}

function scheduleNext(): void {
  clearPollTimer();
  if (!running) return;
  pollTimer = setTimeout(() => {
    void runProbe();
  }, getProbeInterval());
}

function toErrorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  return String(error);
}

async function fetchHealth(controller: AbortController): Promise<void> {
  let timedOut = false;
  const timeout = setTimeout(() => {
    timedOut = true;
    controller.abort();
  }, HEALTH_TIMEOUT_MS);
  try {
    const response = await fetch(`${getApiBase()}/api/health`, {
      method: 'GET',
      signal: controller.signal,
    });
    if (!response.ok) {
      const suffix = response.statusText ? ` ${response.statusText}` : '';
      throw new Error(`HTTP ${response.status}${suffix}`);
    }
  } catch (error) {
    if (error instanceof Error && error.name === 'AbortError') {
      if (!timedOut) throw error;
      throw new Error(`Health check timed out after ${HEALTH_TIMEOUT_MS}ms`);
    }
    throw error;
  } finally {
    clearTimeout(timeout);
  }
}

async function runProbe(allowStopped = false): Promise<void> {
  if (inFlight) return inFlight;
  const controller = new AbortController();
  const generation = probeGeneration;
  activeController = controller;
  const canWriteState = () => generation === probeGeneration && (allowStopped || running);

  inFlight = (async () => {
    try {
      await fetchHealth(controller);
      if (!canWriteState()) return;
      daemonOnline.value = true;
      lastErrorMessage.value = null;
    } catch (error) {
      if (error instanceof Error && error.name === 'AbortError') return;
      if (!canWriteState()) return;
      daemonOnline.value = false;
      lastErrorMessage.value = toErrorMessage(error);
    } finally {
      if (activeController === controller) {
        activeController = null;
      }
      if (canWriteState()) {
        lastCheckedAt.value = Date.now();
      }
      inFlight = null;
      scheduleNext();
    }
  })();
  return inFlight;
}

export function useDaemonHealth() {
  async function forceCheck(): Promise<void> {
    clearPollTimer();
    await runProbe(true);
  }

  function start(): void {
    if (running) return;
    running = true;
    void forceCheck();
  }

  function stop(): void {
    running = false;
    probeGeneration++;
    activeController?.abort();
    activeController = null;
    clearPollTimer();
  }

  return {
    daemonOnline,
    lastCheckedAt,
    lastErrorMessage,
    forceCheck,
    start,
    stop,
  };
}
