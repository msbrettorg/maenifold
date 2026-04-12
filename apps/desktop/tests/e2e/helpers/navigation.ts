import type { Page } from '@playwright/test';
import { pipelinePanel, pipelinesTabButton } from './selectors';

export async function goToPipelineTab(page: Page): Promise<void> {
  await pipelinesTabButton(page).click();
  await pipelinePanel(page).waitFor({ state: 'visible' });
}
