// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { test, expect } from './fixtures';

test.describe('AI Infrastructure OS v17 — Extensions Graduation', () => {
  test('extensions overview API', async ({ request }) => {
    const res = await request.get('/api/intelligence/extensions/overview');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.graduated_count).toBe(10);
    expect(data.era).toBe('L');
  });

  test('extensions chaos experiments API', async ({ request }) => {
    const res = await request.get('/api/intelligence/extensions/chaos/experiments');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.status).toBe('ship');
    expect(data.catalog.experiments.length).toBeGreaterThan(0);
  });

  test('extensions chaos run API', async ({ request }) => {
    const res = await request.post('/api/intelligence/extensions/chaos/run', {
      data: { dry_run: true, experiment_id: 'pod-restart' },
    });
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.status).toBe('ship');
  });

  test('extensions game days API', async ({ request }) => {
    const res = await request.get('/api/intelligence/extensions/game-days');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.plan.scenarios.length).toBeGreaterThan(0);
  });

  test('extensions game day execute API', async ({ request }) => {
    const plan = await request.get('/api/intelligence/extensions/game-days');
    const planBody = await plan.json();
    const planData = planBody.data ?? planBody;
    const scenarioId = planData.plan.scenarios[0].id;
    const res = await request.post('/api/intelligence/extensions/game-days/execute', {
      data: { dry_run: true, scenario_id: scenarioId },
    });
    expect(res.ok()).toBeTruthy();
  });

  test('extensions live activity API', async ({ request }) => {
    const res = await request.get('/api/intelligence/extensions/live-activity');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.status).toBe('ship');
  });

  test('extensions spotlight API', async ({ request }) => {
    const res = await request.get('/api/intelligence/extensions/spotlight');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.indexable_count).toBeGreaterThan(0);
  });

  test('extensions shortcuts API', async ({ request }) => {
    const res = await request.get('/api/intelligence/extensions/shortcuts');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.status).toBe('ship');
  });

  test('extensions menu extras API', async ({ request }) => {
    const res = await request.get('/api/intelligence/extensions/menu-extras');
    expect(res.ok()).toBeTruthy();
  });

  test('extensions native bundle API', async ({ request }) => {
    const res = await request.get('/api/intelligence/extensions/native-bundle');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.status).toBe('ship');
  });

  test('extensions sre bundle API', async ({ request }) => {
    const res = await request.get('/api/intelligence/extensions/sre-bundle');
    expect(res.ok()).toBeTruthy();
  });

  test('extensions graduation panel on observability page', async ({ page }) => {
    await page.goto('/observability');
    await expect(page.getByTestId('extensions-graduation-panel')).toBeVisible({ timeout: 15_000 });
    await page.getByRole('button', { name: 'SRE Chaos' }).click();
    await expect(page.getByTestId('extensions-chaos-panel')).toBeVisible();
  });
});
