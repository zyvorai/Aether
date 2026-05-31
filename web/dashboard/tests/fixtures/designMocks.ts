// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import type { Page, Route } from '@playwright/test';

function json(route: Route, data: unknown, status = 200) {
  return route.fulfill({
    status,
    contentType: 'application/json',
    body: JSON.stringify({ success: true, data }),
  });
}

function text(route: Route, body: string, status = 200) {
  return route.fulfill({
    status,
    contentType: 'text/plain; charset=utf-8',
    body,
  });
}

export async function seedAuthenticatedSession(page: Page) {
  await page.addInitScript(() => {
    sessionStorage.setItem(
      'aether_auth',
      JSON.stringify({
        authenticated: true,
        username: 'admin',
        token: 'e2e-test-token',
        authMode: 'dev',
      }),
    );
  });
}

export async function clearAuthenticatedSession(page: Page) {
  await page.addInitScript(() => {
    sessionStorage.removeItem('aether_auth');
  });
}

export async function installLoginPageMocks(page: Page) {
  await page.route('**/api/**', async (route) => {
    const path = new URL(route.request().url()).pathname.replace(/^\/api/, '');

    if (path === '/auth/me') {
      return json(route, { authenticated: false, username: '' });
    }
    if (path === '/auth/providers') {
      return json(route, { methods: ['bearer'], oidc: { enabled: false } });
    }
    return json(route, {});
  });
}

export async function installDesignApiMocks(page: Page) {
  await page.route('**/api/**', async (route) => {
    const path = new URL(route.request().url()).pathname.replace(/^\/api/, '');

    if (path === '/auth/me') {
      return json(route, { authenticated: true, username: 'admin' });
    }
    if (path === '/auth/providers') {
      return json(route, { methods: ['bearer'], oidc: { enabled: false } });
    }
    if (path === '/metrics') {
      return text(route, '# TYPE up gauge\nup 1\n');
    }
    if (path.startsWith('/workloads')) {
      return json(route, []);
    }
    if (path.startsWith('/cost/chargeback')) {
      return json(route, {
        totalMonthlyUsd: 0,
        totalSpotMonthlyUsd: 0,
        tco36MonthsUsd: 0,
        pricingSource: 'mock',
        region: 'us-east-1',
        lines: [],
      });
    }
    if (path.startsWith('/observability/summary')) {
      return json(route, {
        cluster: { cpu_percent: 0, memory_percent: 0 },
        runtimes: [],
        workloads: [],
      });
    }
    if (path === '/server') {
      return json(route, {
        version: '0.3.0',
        ha_mode: 'standalone',
        integrations: { grafana_url: null, prometheus_url: null },
        kubernetes: { cilium: null },
        opa: { configured: false, enforce: false },
      });
    }
    if (path === '/intelligence/predictions') {
      return json(route, { predictions: [] });
    }
    if (path === '/intelligence/threats') {
      return json(route, { threats: [] });
    }
    if (path === '/intelligence/cost-optimize') {
      return json(route, { recommendations: [], total_savings_usd: 0 });
    }
    if (path === '/intelligence/evolution/status') {
      return json(route, { workloads: [], summary: 'idle' });
    }
    if (path.startsWith('/platform/recommendations')) {
      return json(route, { items: [] });
    }
    if (path.startsWith('/platform')) {
      return json(route, { items: [] });
    }
    if (path.includes('/events/stream')) {
      return route.abort();
    }

    return json(route, {});
  });
}
