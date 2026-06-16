// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

import { chromium, type FullConfig } from '@playwright/test';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const authFile = path.join(__dirname, '../playwright/.auth/user.json');

async function bootstrapAuth(page: import('@playwright/test').Page, baseURL: string) {
  await page.addInitScript(() => {
    localStorage.setItem('zyvor-classic-nav', '1');
    localStorage.setItem('zyvor-pro-view', '0');
  });
  const apiKey = process.env.AETHER_E2E_API_KEY?.trim() ?? process.env.AETHER_API_KEY?.trim();
  const tokenUrl = apiKey ? `${baseURL}/?token=${encodeURIComponent(apiKey)}` : `${baseURL}/`;
  await page.goto(tokenUrl, { waitUntil: 'domcontentloaded' });

  const helpMenu = page.getByRole('button', { name: 'Help menu' });
  if (await helpMenu.isVisible({ timeout: 5000 }).catch(() => false)) {
    return;
  }

  const continueBtn = page.getByRole('button', { name: /continue to dashboard/i });
  if (await continueBtn.isVisible({ timeout: 5000 }).catch(() => false)) {
    await continueBtn.click();
  }

  const samlBtn = page.getByRole('button', { name: /sign in with saml/i });
  if (await samlBtn.isVisible({ timeout: 5000 }).catch(() => false)) {
    await samlBtn.click();
  }

  if (!(await helpMenu.isVisible({ timeout: 8000 }).catch(() => false))) {
    await page.goto(`${baseURL}/?token=`, { waitUntil: 'domcontentloaded' });
    if (await continueBtn.isVisible({ timeout: 5000 }).catch(() => false)) {
      await continueBtn.click();
    }
  }

  await helpMenu.waitFor({ state: 'visible', timeout: 25_000 });
}

export default async function globalSetup(config: FullConfig) {
  fs.mkdirSync(path.dirname(authFile), { recursive: true });
  const baseURL =
    config.projects[0]?.use?.baseURL ??
    process.env.AETHER_E2E_BASE_URL ??
    'http://127.0.0.1:5090';

  const browser = await chromium.launch();
  const context = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  const page = await context.newPage();

  try {
    await bootstrapAuth(page, baseURL);
  } catch (err) {
    console.warn('global-setup: auth bootstrap incomplete — tests may hit login gate:', err);
  }

  await context.storageState({ path: authFile });
  await browser.close();
}
