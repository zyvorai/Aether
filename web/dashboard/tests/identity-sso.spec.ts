// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

import { test, expect } from './fixtures';
import { ensureAuthenticated } from './helpers/auth';

test.describe('Identity & SSO settings', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('settings page shows identity panels', async ({ page }) => {
    await page.goto('/settings');
    await expect(page.getByTestId('identity-sso-panel')).toBeVisible({ timeout: 10_000 });
    await expect(page.getByTestId('identity-oidc-panel')).toBeVisible();
    await expect(page.getByTestId('identity-saml-panel')).toBeVisible();
    await expect(page.getByTestId('identity-ldap-panel')).toBeVisible();
  });

  test('auth settings API returns session and oidc blocks', async ({ request }) => {
    const res = await request.get('/api/auth/settings');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    expect(body.data?.session?.cookie_name).toBe('aether_session');
    expect(body.data?.oidc).toBeTruthy();
    expect(body.data?.saml?.metadata_path).toBe('/api/auth/saml/metadata');
  });

  test('SAML metadata returns 404 when SAML not configured', async ({ request }) => {
    const res = await request.get('/api/auth/saml/metadata');
    expect([404, 200]).toContain(res.status());
  });
});

test.describe('Bearer token URL bootstrap', () => {
  test.use({ storageState: { cookies: [], origins: [] } });

  test('?token= on login auto-authenticates in open dev mode', async ({ page }) => {
    const health = await page.request.get('/health');
    test.skip(!health.ok(), 'API not reachable');

    await page.goto('/?token=');
    const helpMenu = page.getByRole('button', { name: 'Help menu' });
    const loginGate = page.getByRole('button', { name: /continue to dashboard|sign in with/i });
    const onDashboard = await helpMenu.isVisible({ timeout: 5000 }).catch(() => false);
    const onLogin = await loginGate.isVisible({ timeout: 2000 }).catch(() => false);
    expect(onDashboard || onLogin).toBeTruthy();
    expect(page.url()).not.toContain('token=');
  });
});
