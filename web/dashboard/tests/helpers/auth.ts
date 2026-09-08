// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { expect, type Page } from '@playwright/test';

/** Sign in with the seeded demo user (admin / Admin@321) when the login gate is shown. */
export async function ensureAuthenticated(page: Page) {
  await page.setViewportSize({ width: 1280, height: 1400 });
  await page.goto('/');

  const globalNav = page.getByTestId('global-nav');
  if (await globalNav.isVisible({ timeout: 5000 }).catch(() => false)) {
    return;
  }

  const loginGate = page.getByTestId('login-gate');
  if (await loginGate.isVisible({ timeout: 8000 }).catch(() => false)) {
    const user = loginGate.locator('input[name="username"], input[autocomplete="username"]').first();
    const pass = loginGate.locator('input[name="password"], input[type="password"]').first();
    if (await user.isVisible().catch(() => false)) {
      await user.fill('admin');
    }
    if (await pass.isVisible().catch(() => false)) {
      await pass.fill('Admin@321');
    }
    await loginGate.getByRole('button', { name: /sign in|log in|continue/i }).first().click();
    await expect(globalNav).toBeVisible({ timeout: 20_000 });
    return;
  }

  // Legacy continue / SAML paths
  const continueBtn = page.getByRole('button', { name: /continue to dashboard/i });
  if (await continueBtn.isVisible({ timeout: 2000 }).catch(() => false)) {
    await continueBtn.click();
  }

  await expect(globalNav).toBeVisible({ timeout: 20_000 });
}
