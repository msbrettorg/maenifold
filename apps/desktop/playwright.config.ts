import { defineConfig, devices } from '@playwright/test';

const baseURL = process.env.PLAYWRIGHT_BASE_URL ?? 'http://localhost:5173';
const daemonBaseUrl =
  process.env.PLAYWRIGHT_DAEMON_URL ??
  process.env.VITE_DAEMON_URL ??
  'http://127.0.0.1:19761';

process.env.PLAYWRIGHT_DAEMON_URL = daemonBaseUrl;
process.env.VITE_DAEMON_URL = daemonBaseUrl;

export default defineConfig({
  testDir: './tests',
  snapshotPathTemplate: '{testDir}/visual/baselines/{testFilePath}/{arg}{ext}',
  timeout: 60_000,
  workers: 1,
  expect: {
    timeout: 10_000,
  },
  use: {
    baseURL,
    actionTimeout: 30_000,
    navigationTimeout: 30_000,
    screenshot: 'only-on-failure',
    trace: 'retain-on-failure',
  },
  projects: [
    {
      name: 'chromium',
      use: {
        ...devices['Desktop Chrome'],
        browserName: 'chromium',
      },
    },
  ],
  webServer: {
    command: 'npm run dev -- --host 127.0.0.1 --port 5173',
    url: baseURL,
    reuseExistingServer: !process.env.CI,
    stdout: 'pipe',
    stderr: 'pipe',
    timeout: 120_000,
  },
});
