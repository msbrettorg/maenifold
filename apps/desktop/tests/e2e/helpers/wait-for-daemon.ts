import { setTimeout as delay } from 'node:timers/promises';

const READINESS_PATH = '/endophasia';

export interface WaitForDaemonOptions {
  timeoutMs?: number;
  intervalMs?: number;
}

export async function waitForDaemon(
  daemonBaseUrl: string,
  { timeoutMs = 120_000, intervalMs = 500 }: WaitForDaemonOptions = {},
): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  const readinessUrl = new URL(READINESS_PATH, daemonBaseUrl).toString();
  let lastError: string | null = null;

  while (Date.now() < deadline) {
    try {
      const response = await fetch(readinessUrl);
      if (response.ok) {
        return;
      }

      lastError = `HTTP ${response.status} from ${readinessUrl}`;
    } catch (error) {
      lastError = error instanceof Error ? error.message : String(error);
    }

    await delay(intervalMs);
  }

  throw new Error(
    `Timed out waiting for maenifold daemon at ${readinessUrl}${lastError ? ` (${lastError})` : ''}`,
  );
}
