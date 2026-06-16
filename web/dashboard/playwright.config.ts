// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { defineConfig } from '@playwright/test';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const authFile = path.join(__dirname, 'playwright/.auth/user.json');
const baseURL = process.env.AETHER_E2E_BASE_URL ?? 'http://127.0.0.1:5090';

export default defineConfig({
  testDir: './tests',
  timeout: 60_000,
  retries: process.env.CI ? 1 : 0,
  workers: process.env.CI ? 2 : 3,
  globalSetup: './tests/global-setup.ts',
  use: {
    baseURL,
    trace: 'on-first-retry',
    permissions: ['clipboard-read', 'clipboard-write'],
    storageState: authFile,
    viewport: { width: 1280, height: 1400 },
  },
  webServer: process.env.AETHER_E2E_SKIP_SERVER
    ? undefined
    : {
        command:
          'cd ../.. && (cd web/dashboard && npm run build --silent) && cargo build --release --quiet && ./target/release/aether serve --host 127.0.0.1 --port 5090',
        url: `${baseURL}/health`,
        reuseExistingServer: process.env.AETHER_E2E_REUSE_SERVER === '1',
        timeout: 600_000,
      },
});
