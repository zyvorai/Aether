// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

import { describe, expect, it } from 'vitest';
import { ALL_SIDEBAR_ITEMS, SIDEBAR_PRIMARY, SIDEBAR_SECTIONS } from './sidebarNav';
import { DASHBOARD_VIEWS } from './dashboardNav';

describe('sidebarNav', () => {
  it('orders sections as intelligence, operations, resources', () => {
    expect(SIDEBAR_SECTIONS.map((s) => s.id)).toEqual(['intelligence', 'operations', 'resources']);
  });

  it('pins the 4 primary views ungrouped', () => {
    expect(SIDEBAR_PRIMARY.map((v) => v.view)).toEqual(['overview', 'fabric', 'applications', 'workloads']);
  });

  it('includes every dashboard view exactly once', () => {
    expect(ALL_SIDEBAR_ITEMS).toHaveLength(DASHBOARD_VIEWS.length);
    const seen = new Set(ALL_SIDEBAR_ITEMS.map((v) => v.view));
    expect(seen.size).toBe(DASHBOARD_VIEWS.length);
    for (const view of DASHBOARD_VIEWS) {
      expect(seen.has(view.view)).toBe(true);
    }
  });
});
