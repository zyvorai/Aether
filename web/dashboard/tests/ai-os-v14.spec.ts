// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { test, expect } from './fixtures';

test.describe('AI Infrastructure OS v14 — Security & Compliance', () => {
  test('security policy apply API', async ({ request }) => {
    const res = await request.post('/api/intelligence/security/policy-apply', {
      data: { dry_run: true },
    });
    expect(res.ok()).toBeTruthy();
  });

  test('security sbom drift API', async ({ request }) => {
    const res = await request.get('/api/intelligence/security/sbom-drift');
    expect(res.ok()).toBeTruthy();
  });

  test('security zero-trust wizard API', async ({ request }) => {
    const res = await request.get('/api/intelligence/security/zero-trust-wizard');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.steps.length).toBe(3);
  });

  test('security compliance report API', async ({ request }) => {
    const res = await request.get('/api/intelligence/security/compliance-report');
    expect(res.ok()).toBeTruthy();
  });

  test('security rotation agent API', async ({ request }) => {
    const res = await request.post('/api/intelligence/security/rotation-agent', {
      data: { dry_run: true },
    });
    expect(res.ok()).toBeTruthy();
  });

  test('security image enforcement API', async ({ request }) => {
    const res = await request.get('/api/intelligence/security/image-enforcement');
    expect(res.ok()).toBeTruthy();
  });

  test('security threat hunt API', async ({ request }) => {
    const res = await request.post('/api/intelligence/security/threat-hunt', {
      data: { query: 'gpu', limit: 5 },
    });
    expect(res.ok()).toBeTruthy();
  });

  test('security sovereign audit API', async ({ request }) => {
    const res = await request.get('/api/intelligence/security/sovereign-audit');
    expect(res.ok()).toBeTruthy();
  });

  test('security score trend API', async ({ request }) => {
    const res = await request.get('/api/intelligence/security/score-trend');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.points.length).toBeGreaterThan(0);
  });

  test('security platform panel on security page', async ({ page }) => {
    await page.goto('/security');
    await expect(page.getByTestId('security-platform-panel')).toBeVisible({ timeout: 15_000 });
    await expect(page.getByTestId('security-score-panel')).toBeVisible();
    await page.getByRole('button', { name: 'Threat hunt' }).click();
    await expect(page.getByTestId('security-threat-hunt-panel')).toBeVisible();
    await page.getByTestId('security-hunt-button').click();
  });
});
