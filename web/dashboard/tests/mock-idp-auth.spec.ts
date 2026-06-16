// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { test, expect } from './fixtures';

const mockIdpEnabled = process.env.AETHER_MOCK_IDP === '1';

test.describe('Mock IdP SSO flows', () => {
  test.skip(!mockIdpEnabled, 'requires AETHER_MOCK_IDP=1 on the API server');

  test('auth providers shows SAML and OIDC enabled', async ({ request }) => {
    const res = await request.get('/api/auth/providers');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    expect(body.data?.saml?.enabled).toBe(true);
    expect(body.data?.oidc?.enabled).toBe(true);
  });

  test('SAML login completes and reaches dashboard', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('button', { name: /sign in with saml/i }).click();
    await expect(page.getByText(/mock idp user|workloads|overview/i).first()).toBeVisible({
      timeout: 20_000,
    });
  });

  test('SAML encrypted assertion login when mock IdP encrypted mode enabled', async ({ page }) => {
    test.skip(process.env.AETHER_MOCK_IDP_ENCRYPTED !== '1', 'requires AETHER_MOCK_IDP_ENCRYPTED=1');
    await page.goto('/');
    await page.getByRole('button', { name: /sign in with saml/i }).click();
    await expect(page.getByText(/mock idp user|workloads|overview/i).first()).toBeVisible({
      timeout: 20_000,
    });
  });

  test('OIDC login completes and reaches dashboard', async ({ page, context }) => {
    await context.clearCookies();
    await page.goto('/');
    await page.evaluate(() => sessionStorage.clear());
    await page.reload();
    await page.getByRole('button', { name: /sign in with oidc/i }).click();
    await expect(page.getByText(/mock idp user|workloads|overview/i).first()).toBeVisible({
      timeout: 20_000,
    });
  });
});
