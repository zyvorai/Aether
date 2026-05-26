// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { expect, type Page } from '@playwright/test';

export async function ensureAuthenticated(page: Page) {
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto('/');

  const helpMenu = page.getByRole('button', { name: 'Help menu' });
  if (await helpMenu.isVisible({ timeout: 5000 }).catch(() => false)) {
    return;
  }

  const continueBtn = page.getByRole('button', { name: /continue to dashboard/i });
  if (await continueBtn.isVisible({ timeout: 5000 }).catch(() => false)) {
    await continueBtn.click();
  }

  await expect(helpMenu).toBeVisible({ timeout: 20_000 });
}
