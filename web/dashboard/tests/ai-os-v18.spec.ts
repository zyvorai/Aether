// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { test, expect } from './fixtures';

test.describe('AI Infrastructure OS v18 — Production & Trust', () => {
  test('production overview API', async ({ request }) => {
    const res = await request.get('/api/intelligence/production/overview');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.feature_count).toBe(10);
    expect(data.era).toBe('M');
  });

  test('production scorecard API', async ({ request }) => {
    const res = await request.get('/api/intelligence/production/scorecard');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.status).toBe('ship');
    expect(data.checks.length).toBeGreaterThan(0);
  });

  test('production auth plane API', async ({ request }) => {
    const res = await request.get('/api/intelligence/production/auth-plane');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.status).toBe('ship');
  });

  test('production opa plane API', async ({ request }) => {
    const res = await request.get('/api/intelligence/production/opa-plane');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.enforced_paths.length).toBeGreaterThan(0);
  });

  test('production ha plane API', async ({ request }) => {
    const res = await request.get('/api/intelligence/production/ha-plane');
    expect(res.ok()).toBeTruthy();
  });

  test('production durability plane API', async ({ request }) => {
    const res = await request.get('/api/intelligence/production/durability-plane');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.backup_api).toBe('/api/backups');
  });

  test('production hosted plane API', async ({ request }) => {
    const res = await request.get('/api/intelligence/production/hosted-plane');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.endpoints.length).toBeGreaterThan(0);
  });

  test('production edge fleet API', async ({ request }) => {
    const res = await request.get('/api/intelligence/production/edge-fleet');
    expect(res.ok()).toBeTruthy();
  });

  test('production post-deploy manifest API', async ({ request }) => {
    const res = await request.get('/api/intelligence/production/post-deploy-manifest');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.steps.length).toBeGreaterThan(0);
  });

  test('production ci smoke manifest API', async ({ request }) => {
    const res = await request.get('/api/intelligence/production/ci-smoke-manifest');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.jobs.length).toBeGreaterThan(0);
  });

  test('production trust panel on platform page', async ({ page }) => {
    await page.goto('/platform');
    await expect(page.getByTestId('production-trust-panel')).toBeVisible({ timeout: 15_000 });
    await expect(page.getByTestId('production-scorecard-panel')).toBeVisible();
    await page.getByTestId('production-trust-panel').getByRole('button', { name: 'Auth' }).click();
    await expect(page.getByTestId('production-auth-panel')).toBeVisible();
  });
});
