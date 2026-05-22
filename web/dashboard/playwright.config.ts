import { defineConfig } from '@playwright/test';

const baseURL = process.env.AETHER_E2E_BASE_URL ?? 'http://127.0.0.1:5090';

export default defineConfig({
  testDir: './tests',
  timeout: 60_000,
  retries: process.env.CI ? 1 : 0,
  use: {
    baseURL,
    trace: 'on-first-retry',
  },
  webServer: process.env.AETHER_E2E_SKIP_SERVER
    ? undefined
    : {
        command: 'cd ../.. && cargo run --quiet -- serve --host 127.0.0.1 --port 5090',
        url: `${baseURL}/health`,
        reuseExistingServer: true,
        timeout: 120_000,
      },
});
