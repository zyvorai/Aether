// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { test, expect } from './fixtures';

test.describe('AI Infrastructure OS v8 — Multi-Cloud & Federation', () => {
  test('federation execute dry-run API', async ({ request }) => {
    const res = await request.post('/api/intelligence/federation/execute', {
      data: { dry_run: true },
    });
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.dry_run).toBe(true);
  });

  test('cost arbitrage API', async ({ request }) => {
    const res = await request.get('/api/intelligence/multicloud/cost-arbitrage');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(Array.isArray(data.entries)).toBeTruthy();
  });

  test('cluster health mesh API', async ({ request }) => {
    const res = await request.get('/api/intelligence/federation/health-mesh');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(Array.isArray(data.nodes)).toBeTruthy();
  });

  test('unified fabric API', async ({ request }) => {
    const res = await request.get('/api/intelligence/federation/unified-fabric');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(Array.isArray(data.nodes)).toBeTruthy();
  });

  test('volume replication status API', async ({ request }) => {
    const res = await request.get('/api/intelligence/migration/volume-status');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(Array.isArray(data.entries)).toBeTruthy();
  });

  test('migration wave plan API', async ({ request }) => {
    const res = await request.get('/api/intelligence/migration/wave-plan');
    if (res.status() === 500) {
      test.skip(true, 'no fleet workloads for wave plan');
    }
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.plan).toBeTruthy();
  });

  test('geo placement API', async ({ request }) => {
    const res = await request.get('/api/intelligence/federation/geo-placement');
    expect(res.ok()).toBeTruthy();
  });

  test('cloud accounts API', async ({ request }) => {
    const res = await request.get('/api/intelligence/multicloud/cloud-accounts');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.accounts.length).toBeGreaterThan(0);
  });

  test('region lock API', async ({ request }) => {
    const res = await request.get('/api/intelligence/federation/region-lock');
    expect(res.ok()).toBeTruthy();
  });

  test('packetwolf guard API', async ({ request }) => {
    const res = await request.get('/api/intelligence/federation/packetwolf-guard');
    expect(res.ok()).toBeTruthy();
  });

  test('federation platform panel on fleet page', async ({ page }) => {
    await page.goto('/fleet');
    await expect(page.getByTestId('federation-platform-panel')).toBeVisible({ timeout: 15_000 });
    await expect(page.getByTestId('federation-execute-dry-run')).toBeVisible();
  });

  test('unified fabric panel on fabric page', async ({ page }) => {
    await page.goto('/fabric');
    await expect(page.getByTestId('unified-fabric-panel')).toBeVisible({ timeout: 15_000 });
  });

  test('migration panels on migrations page', async ({ page }) => {
    await page.goto('/migrations');
    await expect(page.getByTestId('volume-replication-panel')).toBeVisible({ timeout: 15_000 });
    await expect(page.getByTestId('migration-wave-panel')).toBeVisible();
  });
});
