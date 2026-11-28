// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { describe, expect, it } from 'vitest';
import { pageTitle } from './pageTitle';

describe('pageTitle', () => {
  it('names the page, its parent and the product', () => {
    expect(pageTitle('workloads')).toBe('Workloads · Aether');
    expect(pageTitle('settings-identity')).toBe('Identity & SSO · Settings · Aether');
  });
});
