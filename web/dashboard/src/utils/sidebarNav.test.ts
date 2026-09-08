// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import { ALL_SIDEBAR_ITEMS, SIDEBAR_PRIMARY, SIDEBAR_SECTIONS } from './sidebarNav';

describe('sidebarNav', () => {
  it('orders sections as intelligence, operations, resources', () => {
    expect(SIDEBAR_SECTIONS.map((s) => s.id)).toEqual(['intelligence', 'operations', 'resources']);
  });

  it('keeps a short primary workspace list', () => {
    expect(SIDEBAR_PRIMARY.map((v) => v.view)).toEqual(['overview', 'workloads', 'fabric']);
  });

  it('is hubs-only (Apple-lite density)', () => {
    expect(ALL_SIDEBAR_ITEMS.length).toBeLessThanOrEqual(12);
    expect(ALL_SIDEBAR_ITEMS.map((v) => v.view)).toEqual([
      'overview',
      'workloads',
      'fabric',
      'intelligence',
      'observability',
      'fleet',
      'clusters',
      'migrations',
      'labs',
      'settings',
    ]);
  });

  it('has unique views', () => {
    const seen = new Set(ALL_SIDEBAR_ITEMS.map((v) => v.view));
    expect(seen.size).toBe(ALL_SIDEBAR_ITEMS.length);
  });
});
