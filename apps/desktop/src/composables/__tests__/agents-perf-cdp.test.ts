// @vitest-environment node
import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';

type CdpClient = {
  close(): Promise<void>;
  Runtime: { enable(): Promise<void>; evaluate(args: Record<string, unknown>): Promise<any> };
  Page: { enable(): Promise<void> };
  Performance: { enable(): Promise<void>; getMetrics(): Promise<{ metrics: Array<{ name: string; value: number }> }> };
  Profiler: { enable(): Promise<void>; start(): Promise<void>; stop(): Promise<{ profile?: { samples?: number[] } }> };
};

const CDP_PORT = Number(process.env.CDP_PORT ?? '9232');
const REPO_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../../../../');
const ACP_UI_ROOT = path.resolve(REPO_ROOT, '../acp-ui');
const PERF_KEYS = ['TaskDuration', 'ScriptDuration', 'LayoutCount', 'RecalcStyleCount', 'Nodes', 'JSHeapUsedSize'] as const;
const dynamicImport = new Function('id', 'return import(id)') as (id: string) => Promise<any>;

let client: CdpClient;

async function connectClient(): Promise<CdpClient> {
  const mod = await dynamicImport('chrome-remote-interface');
  const CDP = mod.default ?? mod;
  const targets = await CDP.List({ port: CDP_PORT });
  const target = targets.find((item: any) =>
    item.type === 'page' && (item.title === 'maenifold' || String(item.url ?? '').includes('app.asar/dist/index.html')))
    ?? targets.find((item: any) => item.type === 'page' && item.url !== 'about:blank')
    ?? targets.find((item: any) => item.type === 'page');
  if (!target) throw new Error(`No Electron page target available on CDP port ${CDP_PORT}`);
  return CDP({ port: CDP_PORT, target }) as Promise<CdpClient>;
}

async function evaluate<T>(expression: string): Promise<T> {
  const { result, exceptionDetails } = await client.Runtime.evaluate({
    expression,
    awaitPromise: true,
    returnByValue: true,
  });
  if (exceptionDetails) {
    throw new Error(exceptionDetails.text ?? 'CDP Runtime.evaluate failed');
  }
  return result.value as T;
}

async function requireHarness(): Promise<{ methods: string[]; href: string | null }> {
  return evaluate(`(() => {
    const perf = globalThis.__MAENIFOLD_CDP_PERF__;
    if (!perf) throw new Error('Missing globalThis.__MAENIFOLD_CDP_PERF__; SWE stage must expose the CDP perf harness.');
    return {
      methods: Object.keys(perf).filter((key) => typeof perf[key] === 'function').sort(),
      href: globalThis.location?.href ?? null,
    };
  })()`);
}

async function runHarness<T>(method: string, args: Record<string, unknown> = {}): Promise<T> {
  return evaluate(`(async () => {
    const perf = globalThis.__MAENIFOLD_CDP_PERF__;
    if (!perf || typeof perf.${method} !== 'function') {
      throw new Error('Missing perf harness method: ${method}');
    }
    return await perf.${method}(${JSON.stringify(args)});
  })()`);
}

async function metrics(): Promise<Record<string, number>> {
  const snapshot = await client.Performance.getMetrics();
  return Object.fromEntries(PERF_KEYS.map((name) => [
    name,
    snapshot.metrics.find((metric) => metric.name === name)?.value ?? 0,
  ]));
}

function delta(before: Record<string, number>, after: Record<string, number>): Record<string, number> {
  return Object.fromEntries(PERF_KEYS.map((name) => [name, Number((after[name] - before[name]).toFixed(3))]));
}

async function profiled<T>(label: string, work: () => Promise<T>): Promise<{ result: T; samples: number }> {
  await client.Profiler.start();
  const result = await work();
  const profile = await client.Profiler.stop();
  const samples = profile.profile?.samples?.length ?? 0;
  console.info(`[cdp-perf] profile=${label} samples=${samples}`);
  return { result, samples };
}

describe('CDP Performance Measurement — T-304.3', () => {
  beforeAll(async () => {
    client = await connectClient();
    await Promise.all([
      client.Page.enable(),
      client.Runtime.enable(),
      client.Performance.enable(),
      client.Profiler.enable(),
    ]);
    const harness = await requireHarness();
    console.info(`[cdp-perf] connected port=${CDP_PORT} href=${harness.href ?? 'unknown'} methods=${harness.methods.join(',')}`);
  }, 30_000);

  afterAll(async () => {
    await client?.close();
  });

  it('H2 — isolates sibling re-renders for a single tool_call_update', async () => {
    const before = await metrics();
    const result = await runHarness<{ siblingUpdates: number; targetUpdates: number; toolCallCount: number }>('measureH2', {
      toolCallCount: 10,
      targetIndex: 5,
    });
    const after = await metrics();
    console.info(`[cdp-perf] H2 siblingUpdates=${result.siblingUpdates} targetUpdates=${result.targetUpdates} toolCallCount=${result.toolCallCount} metrics=${JSON.stringify(delta(before, after))}`);
    expect(result.siblingUpdates).toBe(0);
    expect(result.targetUpdates).toBeGreaterThan(0);
  }, 30_000);

  it('H3 — keeps streaming concatenation scaling near-linear', async () => {
    const result = await runHarness<{
      durations: Record<string, number>;
      ratio400to200: number;
      bottleneckConfirmed: boolean;
    }>('measureH3', {
      chunkSizes: [50, 100, 200, 400],
      chunkLength: 50,
    });
    console.info(`[cdp-perf] H3 durations=${JSON.stringify(result.durations)} ratio400to200=${result.ratio400to200.toFixed(3)} bottleneckConfirmed=${result.bottleneckConfirmed}`);
    expect(result.ratio400to200).toBeLessThanOrEqual(2.5);
  }, 30_000);

  it('H4 — renders worst-case mixed-content messages under 500ms', async () => {
    const before = await metrics();
    const { result, samples } = await profiled('H4', () => runHarness<{
      totalRenderMs: number;
      settleFrames: number;
      blockCount: number;
      settled: boolean;
    }>('measureH4'));
    const after = await metrics();
    console.info(`[cdp-perf] H4 totalRenderMs=${result.totalRenderMs.toFixed(2)} settleFrames=${result.settleFrames} blockCount=${result.blockCount} settled=${result.settled} samples=${samples} metrics=${JSON.stringify(delta(before, after))}`);
    expect(result.totalRenderMs).toBeLessThan(500);
    expect(result.settled).toBe(true);
  }, 30_000);

  it('H6 — limits Pinia store mutations to the changed slice', async () => {
    const result = await runHarness<{
      changedSliceSubscribers: number;
      crossStoreMutations: number;
      firedSubscribers: string[];
    }>('measureH6');
    console.info(`[cdp-perf] H6 changedSliceSubscribers=${result.changedSliceSubscribers} crossStoreMutations=${result.crossStoreMutations} fired=${result.firedSubscribers.join(',')}`);
    expect(result.crossStoreMutations).toBe(0);
    expect(result.changedSliceSubscribers).toBeGreaterThan(0);
  }, 30_000);

  it('H7 — keeps average notification rate at or below five notifications per frame', async () => {
    const result = await runHarness<{
      notificationCount: number;
      frames: number;
      avgNotificationsPerFrame: number;
    }>('measureH7', {
      notificationCount: 100,
    });
    console.info(`[cdp-perf] H7 notificationCount=${result.notificationCount} frames=${result.frames} avgNotificationsPerFrame=${result.avgNotificationsPerFrame.toFixed(2)}`);
    expect(result.avgNotificationsPerFrame).toBeLessThanOrEqual(5);
  }, 30_000);

  it('compares our 100-chunk end-to-end cost against dispatcher baseline (informational)', async () => {
    await fs.access(ACP_UI_ROOT);
    const before = await metrics();
    const { result, samples } = await profiled('dispatcher-baseline', () => runHarness<{
      oursMs: number;
      dispatcherBaselineMs: number;
      ratio: number;
    }>('measureAcpUiBaseline', {
      chunkCount: 100,
      chunkLength: 50,
      acpUiRoot: ACP_UI_ROOT,
    }));
    const after = await metrics();
    const verdict = result.ratio <= 1.2 ? 'PASS' : 'INFORMATIONAL-FAIL';
    console.info(`[cdp-perf] baseline oursMs=${result.oursMs.toFixed(2)} dispatcherBaselineMs=${result.dispatcherBaselineMs.toFixed(2)} ratio=${result.ratio.toFixed(3)} verdict=${verdict} samples=${samples} metrics=${JSON.stringify(delta(before, after))}`);
    expect(result.oursMs).toBeGreaterThanOrEqual(0);
    expect(result.dispatcherBaselineMs).toBeGreaterThanOrEqual(0);
  }, 30_000);
});
