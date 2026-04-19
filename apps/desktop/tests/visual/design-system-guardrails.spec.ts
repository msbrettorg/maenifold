import { test, expect, type Page } from '@playwright/test';

async function openTab(page: Page, tabName: 'Agents' | 'Pipelines' | 'Gates' | 'Graph'): Promise<void> {
  await page.getByRole('button', { name: tabName }).click();
}

async function expectSurfaceScreenshot(
  target: Parameters<typeof expect>[0],
  snapshotName: string,
): Promise<void> {
  await expect(target).toHaveScreenshot(snapshotName, {
    animations: 'disabled',
    maxDiffPixelRatio: 0.01,
  });
}

test.describe('Fluent 2 visual baselines', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('navigation', { name: 'Primary navigation' })).toBeVisible();
  });

  test('home dashboard baseline', async ({ page }) => {
    await expectSurfaceScreenshot(page.locator('main'), 'home-dashboard.png');
  });

  test('chat surface baseline', async ({ page }) => {
    const newSessionButton = page.getByRole('button', { name: /^New session$/i });
    if (await newSessionButton.isVisible().catch(() => false)) {
      if (await newSessionButton.isEnabled().catch(() => false)) {
        await newSessionButton.click();
        await page.waitForTimeout(750);
      }
    }

    const chatRegion = page.getByRole('region', { name: 'Chat' });
    const target = await chatRegion.count() > 0 ? chatRegion : page.locator('main');
    await expectSurfaceScreenshot(target, 'chat-surface.png');
  });

  test('pipeline canvas baseline', async ({ page }) => {
    await openTab(page, 'Pipelines');
    await expect(page.locator('.pipeline-view').first()).toBeVisible();
    await expect(page.locator('.pipeline-entry-surface').first()).toBeVisible();
    await expectSurfaceScreenshot(page.locator('main'), 'pipeline-canvas.png');
  });

  test('gate view baseline', async ({ page }) => {
    await openTab(page, 'Gates');
    const gateRegion = page.getByRole('region', { name: 'Gate tasks' });
    await expect(gateRegion).toBeVisible();
    await expectSurfaceScreenshot(gateRegion, 'gate-view.png');
  });

  test('knowledge explorer baseline', async ({ page }) => {
    await openTab(page, 'Graph');
    const knowledgeRegion = page.getByRole('region', { name: 'Knowledge explorer' });
    await expect(knowledgeRegion).toBeVisible();
    await expectSurfaceScreenshot(knowledgeRegion, 'knowledge-explorer.png');
  });
});
