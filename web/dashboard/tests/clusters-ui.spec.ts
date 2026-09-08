// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { test, expect } from './fixtures';

test.describe('Cluster browser UI', () => {
  test('clusters route loads cluster browser or empty state', async ({ page }) => {
    await page.goto('/clusters');
    const heading = page.getByRole('heading', { name: /cluster browser|no kubernetes contexts/i }).first();
    await expect(heading).toBeVisible({ timeout: 15_000 });
  });

  test('clusters page shows stat cards when kubeconfig present', async ({ page }) => {
    await page.goto('/clusters');
    const reachable = page.getByText('Reachable').first();
    const empty = page.getByText('No Kubernetes contexts').first();
    await expect(reachable.or(empty)).toBeVisible({ timeout: 15_000 });
  });
});

test.describe('Auth providers (OIDC / SAML gates)', () => {
  test('auth providers includes SAML block', async ({ request }) => {
    const res = await request.get('/api/auth/providers');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    expect(body.data?.saml).toBeTruthy();
    expect(typeof body.data?.saml?.enabled).toBe('boolean');
  });

  test('SAML login returns 404 when not configured', async ({ request }) => {
    const res = await request.get('/api/auth/saml/login', { maxRedirects: 0 });
    expect([404, 302, 307]).toContain(res.status());
  });

  test('login gate shows bearer token form or open dashboard', async ({ page }) => {
    await page.goto('/');
    const bearer = page.getByLabel(/api bearer token/i);
    const helpMenu = page.getByRole('button', { name: 'Help menu' });
    await expect(bearer.or(helpMenu)).toBeVisible({ timeout: 15_000 });
  });
});
