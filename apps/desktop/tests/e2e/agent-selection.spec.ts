import type { Page } from '@playwright/test';
import { test, expect } from './fixtures/daemon';
import { goToPipelineTab } from './helpers/navigation';
import { usePipelineMcp } from '../../src/composables/usePipelineMcp';

async function cleanupPipelines(daemonBaseUrl: string): Promise<void> {
  const mcp = usePipelineMcp(daemonBaseUrl);
  await mcp.connect();

  try {
    const result = await mcp.listPipelines();
    for (const pipeline of result.pipelines ?? []) {
      await mcp.deletePipeline(pipeline.pipelineId);
    }
  } finally {
    await mcp.disconnect();
  }
}

async function selectAgent(page: Page, agentName: string): Promise<void> {
  const selector = page.locator('#agent-select');
  await expect(selector).toBeVisible();
  await selector.selectOption(agentName);
}

test.beforeEach(async ({ page, daemonBaseUrl }) => {
  await cleanupPipelines(daemonBaseUrl);
  await page.goto('/');
});

test.afterEach(async ({ daemonBaseUrl }) => {
  await cleanupPipelines(daemonBaseUrl);
});

test('creates intent-card pipelines with the last used agent as the primary agent', async ({
  page,
}) => {
  await selectAgent(page, 'Copilot');
  await goToPipelineTab(page);

  await page.getByRole('button', { name: /Research & Verify/i }).click();
  await page.getByRole('button', { name: /^Create Pipeline$/ }).click();

  await expect.poll(async () => page.locator('.vue-flow__node').count()).toBe(2);
  await expect(page.locator('.vue-flow__node').filter({ hasText: 'Copilot' }).first()).toBeVisible();
});

test('changing a prompt node agent stays responsive and updates the node', async ({
  page,
}) => {
  await selectAgent(page, 'Copilot');
  await goToPipelineTab(page);

  const intentInput = page.getByPlaceholder('What do you want to do?');
  await intentInput.fill('Investigate prompt agent switching');
  await intentInput.press('Enter');

  const intentNode = page.getByRole('button', { name: /prompt node: Intent/i });
  await expect(intentNode).toBeVisible();
  await intentNode.dblclick();

  const promptAgentSelect = page.locator('[data-testid="inspector-prompt-agent-select"]');
  await expect(promptAgentSelect).toBeVisible();
  await promptAgentSelect.selectOption('OpenCode');
  await expect(promptAgentSelect).toHaveValue('OpenCode');

  await page.getByRole('button', { name: /Close configuration/i }).click();
  await expect(page.locator('.vue-flow__node').filter({ hasText: 'OpenCode' }).first()).toBeVisible();
});
