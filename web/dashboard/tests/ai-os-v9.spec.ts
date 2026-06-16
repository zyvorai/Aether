// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { test, expect } from './fixtures';

test.describe('AI Infrastructure OS v9 — SRE & Reliability', () => {
  test('runbook schedule API', async ({ request }) => {
    const res = await request.get('/api/intelligence/sre/schedule');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.entries.length).toBeGreaterThan(0);
  });

  test('incident timeline API', async ({ request }) => {
    const res = await request.get('/api/intelligence/sre/incident-timeline');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(Array.isArray(data.entries)).toBeTruthy();
  });

  test('on-call status API', async ({ request }) => {
    const res = await request.get('/api/intelligence/sre/on-call');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.channels.length).toBe(2);
  });

  test('postmortem API', async ({ request }) => {
    const res = await request.get('/api/intelligence/sre/postmortem');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.markdown).toBeTruthy();
  });

  test('error budgets API', async ({ request }) => {
    const res = await request.get('/api/intelligence/sre/error-budgets');
    expect(res.ok()).toBeTruthy();
  });

  test('chaos experiments API', async ({ request }) => {
    const res = await request.get('/api/intelligence/sre/chaos/experiments');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.experiments.length).toBeGreaterThan(0);
  });

  test('game days API', async ({ request }) => {
    const res = await request.get('/api/intelligence/sre/game-days');
    expect(res.ok()).toBeTruthy();
  });

  test('runbook execute dry-run API', async ({ request }) => {
    const res = await request.post('/api/intelligence/sre/runbook/execute', { data: { dry_run: true } });
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.dry_run).toBe(true);
  });

  test('escalation policies API', async ({ request }) => {
    const res = await request.get('/api/intelligence/sre/escalation');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.policies.length).toBeGreaterThan(0);
  });

  test('MTTR API', async ({ request }) => {
    const res = await request.get('/api/intelligence/sre/mttr');
    expect(res.ok()).toBeTruthy();
  });

  test('SRE reliability panel on observability page', async ({ page }) => {
    await page.goto('/observability#obs-reliability');
    await expect(page.getByTestId('sre-reliability-panel')).toBeAttached({ timeout: 15_000 });
    await expect(page.getByTestId('runbook-execute-dry-run')).toBeAttached();
    await page.getByRole('tab', { name: 'Incidents' }).click();
    await expect(page.getByTestId('incident-timeline-panel')).toBeAttached({ timeout: 10_000 });
  });

  test('error budget tab', async ({ page }) => {
    await page.goto('/observability#obs-reliability');
    await expect(page.getByTestId('sre-reliability-panel')).toBeAttached({ timeout: 15_000 });
    await page.getByRole('tab', { name: 'Error budget' }).click();
    await expect(page.getByTestId('error-budget-panel')).toBeAttached({ timeout: 10_000 });
  });
});
