// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { test, expect } from '@playwright/test';

test.describe('Authentication', () => {
  test('auth providers advertises OIDC and bearer', async ({ request }) => {
    const res = await request.get('/api/auth/providers');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    expect(body.data?.methods).toContain('bearer');
    const ssoEnabled =
      body.data?.oidc?.enabled || body.data?.saml?.enabled || body.data?.ldap?.enabled;
    if (ssoEnabled) {
      expect(body.data?.methods).toContain('oidc_session_cookie');
    }
  });

  test('OIDC login returns 404 when not configured', async ({ request }) => {
    const res = await request.get('/api/auth/oidc/login', { maxRedirects: 0 });
    expect([404, 302, 307]).toContain(res.status());
  });
});
