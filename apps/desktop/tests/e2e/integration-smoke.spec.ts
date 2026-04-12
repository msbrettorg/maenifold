import type { Page } from '@playwright/test';
import type { AgentsConfig, MaenifoldSettings } from '../../src/lib/types';
import { usePipelineMcp } from '../../src/composables/usePipelineMcp';
import { test, expect } from './fixtures/daemon';
import { goToPipelineTab } from './helpers/navigation';

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
}

async function fetchConfig(daemonBaseUrl: string): Promise<AgentsConfig> {
  const response = await fetch(new URL('/api/config', daemonBaseUrl).toString());
  if (!response.ok) {
    throw new Error(`Failed to load daemon config: HTTP ${response.status}`);
  }

  return response.json() as Promise<AgentsConfig>;
}

async function fetchConfigPath(daemonBaseUrl: string): Promise<string> {
  const response = await fetch(new URL('/api/config/path', daemonBaseUrl).toString());
  if (!response.ok) {
    throw new Error(`Failed to load daemon config path: HTTP ${response.status}`);
  }

  return response.json() as Promise<string>;
}

async function restoreSettings(
  daemonBaseUrl: string,
  settings: MaenifoldSettings,
): Promise<void> {
  const response = await fetch(new URL('/api/config/settings', daemonBaseUrl).toString(), {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ settings }),
  });

  if (!response.ok) {
    throw new Error(`Failed to restore daemon settings: HTTP ${response.status}`);
  }
}

async function openSettings(page: Page): Promise<void> {
  await expect(page.getByTitle('Settings')).toBeVisible();
  await page.getByTitle('Settings').click();
  await expect(page.getByRole('heading', { name: 'Settings' })).toBeVisible();
}

async function openMaenifoldSettings(page: Page): Promise<void> {
  await page.getByRole('button', { name: /Maenifold/i }).click();
  await expect(page.getByRole('heading', { name: 'Maenifold settings' })).toBeVisible();
}

async function startDraftFromIntent(page: Page): Promise<void> {
  const paletteToggle = page.getByRole('button', { name: '+ Palette' });
  if (await paletteToggle.isVisible().catch(() => false)) {
    return;
  }

  const intentInput = page.getByPlaceholder('What do you want to do?');
  await expect(intentInput).toBeVisible();
  await intentInput.fill('Create an integration smoke pipeline');
  await intentInput.press('Enter');
  await expect(paletteToggle).toBeVisible();
}

async function openPalette(page: Page): Promise<void> {
  if (!(await page.getByRole('button', { name: '✕ Palette' }).isVisible().catch(() => false))) {
    await page.getByRole('button', { name: '+ Palette' }).click();
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

test.describe('pipeline integration smoke', () => {
  const createdPipelineIds = new Set<string>();

  test.afterEach(async ({ daemonBaseUrl }) => {
    await cleanupPipelines(daemonBaseUrl, createdPipelineIds);
    createdPipelineIds.clear();
  });

  test('config loads and settings show daemon-served config', async ({ page, daemonBaseUrl }) => {
    const [config, configPath] = await Promise.all([
      fetchConfig(daemonBaseUrl),
      fetchConfigPath(daemonBaseUrl),
    ]);

    await openApp(page);
    await openSettings(page);

    await expect(page.getByText(configPath)).toBeVisible();

    await openMaenifoldSettings(page);
    await expect(page.getByLabel('Debounce ms')).toHaveValue(String(config.settings?.debounceMs ?? 150));
    await expect(page.getByLabel('Log rotation')).toHaveValue(config.settings?.logRotation ?? 'daily');
  });

  test('settings save round-trip persists across reload', async ({ page, daemonBaseUrl }) => {
    const config = await fetchConfig(daemonBaseUrl);
    const originalSettings = config.settings;
    if (!originalSettings) {
      throw new Error('Daemon did not return settings to round-trip.');
    }

    const nextDebounceValue = originalSettings.debounceMs + 7;

    try {
      await openApp(page);
      await openSettings(page);
      await openMaenifoldSettings(page);

      const debounceInput = page.getByLabel('Debounce ms');
      await debounceInput.fill(String(nextDebounceValue));
      await page.getByRole('button', { name: 'Save' }).click();

      await expect(page.getByText('Maenifold settings saved.')).toBeVisible();
      await expect(debounceInput).toHaveValue(String(nextDebounceValue));

      await page.reload();
      await openSettings(page);
      await openMaenifoldSettings(page);
      await expect(page.getByLabel('Debounce ms')).toHaveValue(String(nextDebounceValue));
    } finally {
      await restoreSettings(daemonBaseUrl, originalSettings);
    }
  });

  test('pipeline full lifecycle renders server mode', async ({ page, daemonBaseUrl }) => {
    await openApp(page);
    await goToPipelineTab(page);
    await startDraftFromIntent(page);
    await addWorkflowNodeFromPalette(page);

    await expect(page.locator('.vue-flow__node')).toHaveCount(2);

    await dragBetweenHandles(
      page,
      '.vue-flow__node:nth-of-type(1) .vue-flow__handle-right',
      '.vue-flow__node:nth-of-type(2) .vue-flow__handle-left',
    );

    await expect(page.locator('.vue-flow__edge')).toHaveCount(1);

    const pipelineId = await createPipelineFromDraft(page, daemonBaseUrl);
    createdPipelineIds.add(pipelineId);

    await expect(page.locator('.status-line')).toContainText('● active');
    await expect(page.getByRole('button', { name: '+ Palette' })).toHaveCount(0);
    await expect(page.locator('.vue-flow__edge')).toHaveCount(1);
  });
});
