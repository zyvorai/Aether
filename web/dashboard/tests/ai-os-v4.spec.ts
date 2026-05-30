// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { test, expect } from '@playwright/test';

test.describe('AI Infrastructure OS v4', () => {
  test('intent pipeline API', async ({ request }) => {
    const res = await request.post('/api/intelligence/intent-pipeline', {
      data: { goals: ['cost'], workload_name: 'demo' },
    });
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.workload_name).toBe('demo');
    expect(data.steps.length).toBeGreaterThan(0);
  });

  test('sre runbook API', async ({ request }) => {
    const res = await request.get('/api/intelligence/sre/runbook');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.runbook_markdown).toContain('# Aether SRE Runbook');
  });

  test('multicloud posture API', async ({ request }) => {
    const res = await request.get('/api/intelligence/multicloud/posture');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(Array.isArray(data.clusters)).toBeTruthy();
  });

  test('autonomous placement API', async ({ request }) => {
    const res = await request.get('/api/intelligence/autonomous/placement');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(Array.isArray(data.workloads)).toBeTruthy();
  });

  test('intent pipeline panel on AI page', async ({ page }) => {
    await page.goto('/ai?tab=pipeline');
    await expect(page.getByTestId('intent-pipeline-panel')).toBeVisible({ timeout: 15_000 });
  });

  test('autonomous SRE panel on observability', async ({ page }) => {
    await page.goto('/observability');
    await expect(page.getByTestId('autonomous-sre-panel')).toBeVisible({ timeout: 15_000 });
  });

  test('multicloud panel on fleet', async ({ page }) => {
    await page.goto('/fleet');
    await expect(page.getByTestId('multicloud-panel')).toBeVisible({ timeout: 15_000 });
  });

  test('autonomous placement on migrations', async ({ page }) => {
    await page.goto('/migrations');
    await expect(page.getByTestId('autonomous-placement-panel')).toBeVisible({ timeout: 15_000 });
  });
});
