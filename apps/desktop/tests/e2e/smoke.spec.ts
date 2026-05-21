import { test, expect } from './fixtures/daemon';
import { goToPipelineTab } from './helpers/navigation';
import { appSelectors, pipelinePanel, primaryNavigation } from './helpers/selectors';

test('app loads and renders the primary shell', async ({ page, daemonBaseUrl }) => {
  await page.goto('/');

  await expect(primaryNavigation(page)).toBeVisible();
  await expect(page.getByRole('heading', { name: appSelectors.welcomeHeading })).toBeVisible();

  await goToPipelineTab(page);
  await expect(pipelinePanel(page)).toBeVisible();

  await expect.poll(async () => {
    const response = await fetch(new URL('/endophasia', daemonBaseUrl).toString());
    return response.ok;
  }).toBe(true);
});
