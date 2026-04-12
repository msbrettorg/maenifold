import type { Page } from '@playwright/test';
import { test, expect } from './fixtures/daemon';
import { goToPipelineTab } from './helpers/navigation';
import { agentsTabButton } from './helpers/selectors';
import { usePipelineMcp } from '../../src/composables/usePipelineMcp';

type PipelineMcp = ReturnType<typeof usePipelineMcp>;

const TEMPLATE = {
  researchAndVerify: {
    label: 'Research & Verify',
    nodeCount: 2,
    edgeCount: 1,
    labels: ['Investigate Topic', 'Peer Review'],
  },
  draftAndRefine: {
    label: 'Draft & Refine',
    nodeCount: 3,
    edgeCount: 2,
    labels: ['Initial Draft', 'Stress Test', 'Refined Output'],
  },
} as const;

function escapeRegex(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

async function withPipelineMcp<T>(
  daemonBaseUrl: string,
  run: (mcp: PipelineMcp) => Promise<T>,
): Promise<T> {
  const mcp = usePipelineMcp(daemonBaseUrl);
  await mcp.connect();

  try {
    return await run(mcp);
  } finally {
    await mcp.disconnect();
  }
}

async function listPipelineIds(mcp: PipelineMcp): Promise<string[]> {
  const result = await mcp.listPipelines();
  return (result.pipelines ?? []).map((pipeline) => pipeline.pipelineId);
}

async function cleanupPipelines(daemonBaseUrl: string): Promise<void> {
  await withPipelineMcp(daemonBaseUrl, async (mcp) => {
    for (const pipelineId of await listPipelineIds(mcp)) {
      await mcp.deletePipeline(pipelineId);
    }
  });
}

async function waitForNewPipelineId(
  mcp: PipelineMcp,
  existingIds: Set<string>,
): Promise<string> {
  for (let attempt = 0; attempt < 40; attempt += 1) {
    const ids = await listPipelineIds(mcp);
    const created = ids.filter((id) => !existingIds.has(id));
    if (created.length > 0) {
      return created[created.length - 1]!;
    }

    await new Promise((resolve) => setTimeout(resolve, 250));
  }

  throw new Error('Timed out waiting for pipeline creation');
}

async function openPipelineFlyout(page: Page): Promise<void> {
  const toggle = page.getByRole('button', { name: /pipelines flyout/i });
  if ((await toggle.getAttribute('aria-expanded')) !== 'true') {
    await toggle.click();
  }

  await expect(page.locator('#pipeline-flyout')).toBeVisible();
}

async function expectCanvasCounts(
  page: Page,
  nodeCount: number,
  edgeCount: number,
): Promise<void> {
  await expect.poll(async () => page.locator('.vue-flow__node').count()).toBe(nodeCount);
  await expect.poll(async () => page.locator('.vue-flow__edge').count()).toBe(edgeCount);
}

async function createPipelineFromTemplate(
  page: Page,
  mcp: PipelineMcp,
  template: (typeof TEMPLATE)[keyof typeof TEMPLATE],
): Promise<string> {
  const beforeIds = new Set(await listPipelineIds(mcp));

  await expect(page.getByRole('heading', { name: /What would you like to do\?/i })).toBeVisible();
  await page.getByRole('button', { name: new RegExp(escapeRegex(template.label), 'i') }).click();
  await page.getByRole('button', { name: /^Create Pipeline$/ }).click();

  await expectCanvasCounts(page, template.nodeCount, template.edgeCount);
  await expect(page.getByRole('toolbar', { name: /Pipeline actions/i })).toBeVisible();
  await page.getByRole('button', { name: /^Create$/ }).click();

  const pipelineId = await waitForNewPipelineId(mcp, beforeIds);
  await expectCanvasCounts(page, template.nodeCount, template.edgeCount);

  for (const label of template.labels) {
    await expect(page.getByText(label, { exact: true })).toBeVisible();
  }

  return pipelineId;
}

test.beforeEach(async ({ page, daemonBaseUrl }) => {
  await cleanupPipelines(daemonBaseUrl);
  await page.goto('/');
  await goToPipelineTab(page);
});

test.afterEach(async ({ daemonBaseUrl }) => {
  await cleanupPipelines(daemonBaseUrl);
});

test('empty state shows a centered empty pipeline experience with create affordances', async ({ page }) => {
  await expect(page.getByRole('heading', { name: /What would you like to do\?/i })).toBeVisible();
  await expect(page.getByRole('button', { name: /Create new pipeline/i })).toBeVisible();

  await openPipelineFlyout(page);
  await expect(page.getByText('No pipelines yet')).toBeVisible();
  await expect(page.getByRole('button', { name: /\+ New Pipeline/i })).toBeVisible();
});

test('create pipeline from an intent card and render it on the canvas', async ({ page, daemonBaseUrl }) => {
  await withPipelineMcp(daemonBaseUrl, async (mcp) => {
    await createPipelineFromTemplate(page, mcp, TEMPLATE.researchAndVerify);
  });
});

test('load an existing pipeline after navigating away and back', async ({ page, daemonBaseUrl }) => {
  await withPipelineMcp(daemonBaseUrl, async (mcp) => {
    const pipelineId = await createPipelineFromTemplate(page, mcp, TEMPLATE.draftAndRefine);

    await page.getByRole('button', { name: /Create new pipeline/i }).click();
    await expect(page.getByRole('heading', { name: /What would you like to do\?/i })).toBeVisible();

    await agentsTabButton(page).click();
    await goToPipelineTab(page);

    await openPipelineFlyout(page);
    await page.locator(`[data-pipeline-id="${pipelineId}"]`).click();

    await expectCanvasCounts(page, TEMPLATE.draftAndRefine.nodeCount, TEMPLATE.draftAndRefine.edgeCount);
    await expect(page.getByText('Stress Test', { exact: true })).toBeVisible();
    await expect(page.getByText('Refined Output', { exact: true })).toBeVisible();
  });
});

test('delete a pipeline from the action bar and return to the empty state', async ({ page, daemonBaseUrl }) => {
  await withPipelineMcp(daemonBaseUrl, async (mcp) => {
    const pipelineId = await createPipelineFromTemplate(page, mcp, TEMPLATE.researchAndVerify);

    await mcp.callPipeline({ sessionId: pipelineId, forceComplete: true });
    await expect(page.getByRole('button', { name: /^Delete$/ })).toBeVisible({ timeout: 15_000 });

    page.once('dialog', (dialog) => dialog.accept());
    await page.getByRole('button', { name: /^Delete$/ }).click();

    await expect(page.getByRole('heading', { name: /What would you like to do\?/i })).toBeVisible();
    await openPipelineFlyout(page);
    await expect(page.getByText('No pipelines yet')).toBeVisible();
  });
});

test('pipeline list supports multiple pipelines and selection switching', async ({ page, daemonBaseUrl }) => {
  await withPipelineMcp(daemonBaseUrl, async (mcp) => {
    const firstPipelineId = await createPipelineFromTemplate(page, mcp, TEMPLATE.researchAndVerify);

    await page.getByRole('button', { name: /Create new pipeline/i }).click();
    const secondPipelineId = await createPipelineFromTemplate(page, mcp, TEMPLATE.draftAndRefine);

    await openPipelineFlyout(page);
    await expect(page.locator('[data-pipeline-item]')).toHaveCount(2);

    await page.locator(`[data-pipeline-id="${firstPipelineId}"]`).click();
    await expectCanvasCounts(page, TEMPLATE.researchAndVerify.nodeCount, TEMPLATE.researchAndVerify.edgeCount);
    await expect(page.getByText('Peer Review', { exact: true })).toBeVisible();

    await openPipelineFlyout(page);
    await page.locator(`[data-pipeline-id="${secondPipelineId}"]`).click();
    await expectCanvasCounts(page, TEMPLATE.draftAndRefine.nodeCount, TEMPLATE.draftAndRefine.edgeCount);
    await expect(page.getByText('Stress Test', { exact: true })).toBeVisible();
  });
});
