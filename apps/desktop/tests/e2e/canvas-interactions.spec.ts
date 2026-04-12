import type { Page } from '@playwright/test';
import { test, expect } from './fixtures/daemon';
import { goToPipelineTab } from './helpers/navigation';
import { usePipelineMcp } from '../../src/composables/usePipelineMcp';

const STORAGE_RESET_SCRIPT = () => {
  for (const key of Object.keys(window.localStorage)) {
    if (key.startsWith('pipeline-positions-v3-') || key === 'pipeline-node-palette-v1') {
      window.localStorage.removeItem(key);
    }
  }
};

async function openApp(page: Page): Promise<void> {
  await page.addInitScript(STORAGE_RESET_SCRIPT);
  await page.goto('/');
  await goToPipelineTab(page);
}

async function startDraftFromIntent(page: Page): Promise<void> {
  const paletteToggle = page.getByRole('button', { name: '+ Palette' });
  if (await paletteToggle.isVisible().catch(() => false)) {
    return;
  }

  const blockedWarning = page.getByText(/Configure an agent before creating a pipeline/i);
  if (await blockedWarning.isVisible().catch(() => false)) {
    throw new Error('E2E pipeline tests require at least one configured agent.');
  }

  const intentInput = page.getByPlaceholder('What do you want to do?');
  await expect(intentInput).toBeVisible();
  await intentInput.fill('Create an E2E pipeline');
  await intentInput.press('Enter');
  await expect(paletteToggle).toBeVisible();
}

async function openPalette(page: Page): Promise<void> {
  const paletteToggle = page.getByRole('button', { name: '+ Palette' });
  if (!(await page.getByRole('button', { name: '✕ Palette' }).isVisible().catch(() => false))) {
    await paletteToggle.click();
  }

  await expect(page.getByRole('heading', { name: 'Add Node' })).toBeVisible();
}

async function addWorkflowNodeFromPalette(page: Page): Promise<void> {
  await openPalette(page);
  const workflowCard = page.getByRole('button', { name: /Run a Workflow/i }).last();
  await expect(workflowCard).toBeEnabled();
  await workflowCard.click();
}

async function listPipelineIds(daemonBaseUrl: string): Promise<string[]> {
  const mcp = usePipelineMcp(daemonBaseUrl);
  if (!mcp.connected.value) {
    await mcp.connect();
  }

  const result = await mcp.listPipelines();
  return (result.pipelines ?? []).map((pipeline) => pipeline.pipelineId);
}

async function createPipelineFromDraft(
  page: Page,
  daemonBaseUrl: string,
): Promise<string> {
  const before = new Set(await listPipelineIds(daemonBaseUrl));
  await page.getByRole('button', { name: 'Create' }).click();

  await expect(page.getByRole('button', { name: '+ Palette' })).toHaveCount(0);

  let pipelineId: string | null = null;
  await expect.poll(async () => {
    const ids = await listPipelineIds(daemonBaseUrl);
    pipelineId = ids.find((id) => !before.has(id)) ?? null;
    return pipelineId;
  }).not.toBeNull();

  return pipelineId!;
}

async function cleanupPipelines(daemonBaseUrl: string, pipelineIds: Iterable<string>): Promise<void> {
  const mcp = usePipelineMcp(daemonBaseUrl);
  if (!mcp.connected.value) {
    await mcp.connect();
  }

  for (const pipelineId of pipelineIds) {
    try {
      await mcp.deletePipeline(pipelineId);
    } catch {
      // Best-effort cleanup against the live daemon.
    }
  }
}

async function dragBetweenHandles(
  page: Page,
  sourceHandleSelector: string,
  targetHandleSelector: string,
): Promise<void> {
  const sourceHandle = page.locator(sourceHandleSelector);
  const targetHandle = page.locator(targetHandleSelector);

  await expect(sourceHandle).toBeVisible();
  await expect(targetHandle).toBeVisible();

  const sourceBox = await sourceHandle.boundingBox();
  const targetBox = await targetHandle.boundingBox();

  if (!sourceBox || !targetBox) {
    throw new Error('Unable to resolve connection handle positions.');
  }

  await page.mouse.move(sourceBox.x + sourceBox.width / 2, sourceBox.y + sourceBox.height / 2);
  await page.mouse.down();
  await page.mouse.move(targetBox.x + targetBox.width / 2, targetBox.y + targetBox.height / 2, {
    steps: 12,
  });
  await page.mouse.up();
}

test.describe('pipeline canvas interactions', () => {
  const createdPipelineIds = new Set<string>();

  test.afterEach(async ({ daemonBaseUrl }) => {
    await cleanupPipelines(daemonBaseUrl, createdPipelineIds);
    createdPipelineIds.clear();
  });

  test('single-click selects node', async ({ page }) => {
    await openApp(page);
    await startDraftFromIntent(page);

    await page.getByRole('button', { name: /prompt node: Intent/i }).click();

    await expect(page.getByRole('button', { name: 'Delete Selected' })).toBeVisible();
    await expect(page.locator('.vue-flow__node.selected')).toHaveCount(1);
  });

  test('double-click opens zoom overlay', async ({ page, daemonBaseUrl }) => {
    await openApp(page);
    await startDraftFromIntent(page);

    const pipelineId = await createPipelineFromDraft(page, daemonBaseUrl);
    createdPipelineIds.add(pipelineId);

    await page.getByRole('button', { name: /prompt node: Intent/i }).dblclick();

    await expect(page.locator('.zoom-overlay')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Close inspection' })).toBeVisible();
  });

  test('right-click shows context menu', async ({ page }) => {
    await openApp(page);
    await startDraftFromIntent(page);

    await page.getByRole('button', { name: /prompt node: Intent/i }).click({ button: 'right' });

    await expect(page.locator('.composer-context-menu')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Edit' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Duplicate' })).toBeVisible();
  });

  test('action bar shows correct buttons per pipeline state', async ({ page, daemonBaseUrl }) => {
    const mcp = usePipelineMcp(daemonBaseUrl);
    await mcp.connect();

    await openApp(page);
    await startDraftFromIntent(page);

    const actionBar = page.getByRole('toolbar', { name: 'Pipeline actions' });
    await expect(actionBar).toBeVisible();
    await expect(actionBar.getByRole('button', { name: 'Create' })).toBeVisible();
    await expect(actionBar.getByRole('button', { name: 'Pause' })).toHaveCount(0);

    const pipelineId = await createPipelineFromDraft(page, daemonBaseUrl);
    createdPipelineIds.add(pipelineId);

    await expect(page.getByRole('toolbar', { name: 'Pipeline actions' })).toHaveCount(0);

    await mcp.runPipeline(pipelineId);
    await expect(page.getByRole('button', { name: 'Pause' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Cancel' })).toBeVisible();

    await mcp.pausePipeline(pipelineId);
    await expect(page.getByRole('button', { name: 'Resume' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Cancel' })).toBeVisible();
  });

  test('add node from palette', async ({ page }) => {
    await openApp(page);
    await startDraftFromIntent(page);

    await addWorkflowNodeFromPalette(page);

    await expect(page.locator('.vue-flow__node')).toHaveCount(2);
    await expect(page.locator('.status-line')).toContainText('Design mode — 2 nodes, 0 edges');
  });

  test('connect nodes by dragging a handle', async ({ page }) => {
    await openApp(page);
    await startDraftFromIntent(page);
    await addWorkflowNodeFromPalette(page);

    await expect(page.locator('.vue-flow__node')).toHaveCount(2);

    await dragBetweenHandles(
      page,
      '.vue-flow__node:nth-of-type(1) .vue-flow__handle-right',
      '.vue-flow__node:nth-of-type(2) .vue-flow__handle-left',
    );

    await expect(page.locator('.vue-flow__edge')).toHaveCount(1);
    await expect(page.locator('.status-line')).toContainText('Design mode — 2 nodes, 1 edges');
  });
});
