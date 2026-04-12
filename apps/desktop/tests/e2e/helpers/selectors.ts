import type { Locator, Page } from '@playwright/test';

export const appSelectors = {
  primaryNavigationName: 'Primary navigation',
  agentsTabName: 'Agents',
  pipelinesTabName: 'Pipelines',
  gatesTabName: 'Gates',
  workflowsTabName: 'Workflows',
  graphTabName: 'Graph',
  welcomeHeading: /Welcome to maenifold/i,
  pipelinePanel: '.pipeline-panel',
} as const;

export function primaryNavigation(page: Page): Locator {
  return page.getByRole('navigation', { name: appSelectors.primaryNavigationName });
}

export function agentsTabButton(page: Page): Locator {
  return page.getByRole('button', { name: appSelectors.agentsTabName });
}

export function pipelinesTabButton(page: Page): Locator {
  return page.getByRole('button', { name: appSelectors.pipelinesTabName });
}

export function gatesTabButton(page: Page): Locator {
  return page.getByRole('button', { name: appSelectors.gatesTabName });
}

export function workflowsTabButton(page: Page): Locator {
  return page.getByRole('button', { name: appSelectors.workflowsTabName });
}

export function graphTabButton(page: Page): Locator {
  return page.getByRole('button', { name: appSelectors.graphTabName });
}

export function pipelinePanel(page: Page): Locator {
  return page.locator(appSelectors.pipelinePanel);
}
