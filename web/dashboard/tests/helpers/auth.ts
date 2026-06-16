// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { expect, type Page } from '@playwright/test';

export async function ensureAuthenticated(page: Page) {
  await page.addInitScript(() => {
    localStorage.setItem('zyvor-classic-nav', '1');
    localStorage.setItem('zyvor-pro-view', '0');
  });
  await page.setViewportSize({ width: 1280, height: 1400 });
  await page.goto('/');

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

  await expect(helpMenu).toBeVisible({ timeout: 20_000 });
}
