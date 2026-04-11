const { mkdirSync } = require('node:fs');
const path = require('node:path');
const { test, expect } = require('@playwright/test');

const baseUrl = process.env.T1253_EVIDENCE_URL ?? 'http://127.0.0.1:5173/evidence/T-125.3/';
const screenshotDir = path.join(
  process.cwd(),
  'src/components/__tests__/evidence/T-125.3/screenshots',
);

function screenshotPath(name) {
  return path.join(screenshotDir, name);
}

async function prepareToolPalette(page) {
  await page.goto(baseUrl);
  await page.waitForLoadState('networkidle');
  await page.getByLabel('Node type').selectOption('tool');
}

test.beforeAll(() => {
  mkdirSync(screenshotDir, { recursive: true });
});

test.beforeEach(async ({ page }) => {
  await page.setViewportSize({ width: 1680, height: 1320 });
});

test('captures search-first flat results with highlight and count', async ({ page }) => {
  await prepareToolPalette(page);
  await page.getByRole('searchbox', { name: 'Search tools' }).fill('mem');

  await expect(page.getByText('4 results')).toBeVisible();
  await expect(page.locator('mark').first()).toBeVisible();

  await page.locator('.evidence-layout').screenshot({
    path: screenshotPath('search-results-highlight-count.png'),
  });
});

test('captures palette staying open after add with compact selected-tool editor', async ({ page }) => {
  await prepareToolPalette(page);
  await page.getByRole('searchbox', { name: 'Search tools' }).fill('mem');
  await expect(page.getByText('SearchMemories')).toBeVisible();

  const queryInput = page.getByRole('textbox', { name: 'Query' });
  await queryInput.fill('alpha memo');

  const addButton = page.getByRole('button', { name: 'Add Tool' }).last();
  await expect(addButton).toBeEnabled();
  await addButton.click();

  await expect(page.getByText('maenifold.SearchMemories (query=alpha memo)')).toBeVisible();

  await page.locator('.evidence-layout').screenshot({
    path: screenshotPath('palette-persists-after-add.png'),
  });
});

test('captures keyboard-oriented listbox navigation state', async ({ page }) => {
  await prepareToolPalette(page);
  await page.getByRole('searchbox', { name: 'Search tools' }).fill('mem');
  const listbox = page.getByRole('listbox', { name: 'Tool search results' });

  await listbox.focus();
  await page.keyboard.press('ArrowDown');
  await expect(page.locator('.tool-palette__option.is-selected')).toContainText('WriteMemory');

  await listbox.screenshot({
    path: screenshotPath('keyboard-listbox-navigation.png'),
  });
});
