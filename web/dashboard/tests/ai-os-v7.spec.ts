// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { test, expect } from '@playwright/test';

test.describe('AI Infrastructure OS v7 — Intent Platform', () => {
  test('NL intent parse API', async ({ request }) => {
    const res = await request.post('/api/intelligence/intent/nl-parse', {
      data: { text: 'cost optimized API under $200 per month', workload_name: 'api' },
    });
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.intent_yaml).toBeTruthy();
  });

  test('intent pipeline deploy dry-run API', async ({ request }) => {
    const res = await request.post('/api/intelligence/intent-pipeline/deploy', {
      data: { goals: ['cost'], workload_name: 'demo', dry_run: true },
    });
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.dry_run).toBe(true);
    expect(data.steps.length).toBeGreaterThan(0);
  });

  test('intent violations API', async ({ request }) => {
    const res = await request.get('/api/intelligence/intent/violations');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(Array.isArray(data.violations)).toBeTruthy();
  });

  test('intent templates API', async ({ request }) => {
    const res = await request.get('/api/intelligence/intent/templates');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.templates.length).toBeGreaterThan(0);
  });

  test('intent bundles API', async ({ request }) => {
    const res = await request.post('/api/intelligence/intent/bundles', {
      data: { bundle_name: 'shop', goal: 'balanced' },
    });
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.workloads.length).toBe(3);
  });

  test('intent gitops diff API', async ({ request }) => {
    const res = await request.get('/api/intelligence/intent/gitops-diff');
    expect(res.ok()).toBeTruthy();
  });

  test('intent platform panel on AI page', async ({ page }) => {
    await page.goto('/ai?tab=intent');
    await expect(page.getByTestId('intent-platform-panel')).toBeVisible({ timeout: 15_000 });
  });

  test('NL intent input on pipeline tab', async ({ page }) => {
    await page.goto('/ai?tab=pipeline');
    await expect(page.getByTestId('intent-nl-input')).toBeVisible({ timeout: 15_000 });
    await expect(page.getByTestId('intent-pipeline-deploy-dry-run')).toBeVisible();
  });

  test('intent gitops diff panel', async ({ page }) => {
    await page.goto('/gitops');
    await expect(page.getByTestId('intent-gitops-diff-panel')).toBeVisible({ timeout: 15_000 });
  });
});
