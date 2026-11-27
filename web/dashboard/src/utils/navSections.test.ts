// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { describe, expect, it } from 'vitest';
import { DASHBOARD_VIEWS } from './dashboardNav';
import {
  NAV_SECTIONS,
  buildNavTree,
  filterViews,
  getChildViews,
  getParentView,
  getRootView,
  getSectionId,
  getSectionTabs,
} from './navSections';

const tree = buildNavTree();
const inTree = new Set(tree.flatMap((section) => section.items.map((item) => item.view)));

describe('navigation sections', () => {
  it('lists only registered views, each once', () => {
    const listed = NAV_SECTIONS.flatMap((section) => section.views);
    expect(new Set(listed).size).toBe(listed.length);
    const registered = new Set(DASHBOARD_VIEWS.map((v) => v.view));
    expect(listed.filter((view) => !registered.has(view))).toEqual([]);
  });

  it('keeps every registered page reachable from the sidebar, directly or through its parent', () => {
    const unreachable = DASHBOARD_VIEWS.filter(
      (meta) => meta.sidebar !== false && !inTree.has(getRootView(meta.view)),
    ).map((meta) => meta.view);
    expect(unreachable).toEqual([]);
  });

  it('derives parents from paths', () => {
    expect(getParentView('security-sbom')).toBe('security');
    expect(getParentView('ai-providers')).toBe('settings');
    expect(getParentView('security')).toBeNull();
    expect(getParentView('overview')).toBeNull();
    expect(getRootView('gitops-sync')).toBe('gitops');
    expect(getSectionId('security-sbom')).toBe('security');
  });

  it('gives hub pages and their children the same tab strip', () => {
    const fromParent = getSectionTabs('security').map((v) => v.view);
    expect(fromParent[0]).toBe('security');
    expect(fromParent).toContain('security-sbom');
    expect(getSectionTabs('security-sbom').map((v) => v.view)).toEqual(fromParent);
    expect(getChildViews('security').length).toBe(fromParent.length - 1);
    expect(getSectionTabs('audit')).toEqual([]);
  });

  it('filters across child pages and puts name matches first', () => {
    expect(filterViews('')).toEqual([]);
    expect(filterViews('sbom')[0].view).toBe('security-sbom');
    expect(filterViews('audit')[0].view).toBe('audit');
    expect(filterViews('zzzz-nothing')).toEqual([]);
  });
});
