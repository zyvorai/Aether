// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { createElement } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { MemoryRouter } from 'react-router';
import { describe, expect, it } from 'vitest';
import SectionHubPage from '../components/SectionHubPage';
import SectionTabs from '../components/SectionTabs';
import type { AppView } from '../types/api';
import { DASHBOARD_VIEWS } from './dashboardNav';
import { pathToView } from './dashboardRoutes';
import { HUB_FILTERS, SHOW_HUB_STATE, hubRedirect, hubTarget } from './hubLinks';
import { getChildViews, getParentView } from './navSections';

/** Where an address ends up, as the app decides it. */
function open(address: string, state?: unknown): string {
  const url = new URL(address, 'http://aether.test');
  const view = pathToView(url.pathname);
  if (!view) throw new Error(`no page at ${url.pathname}`);
  const to = hubRedirect(view, { search: url.search, hash: url.hash, state });
  return to ? `${to.pathname}${to.search}${to.hash}` : address;
}

const HUBS = DASHBOARD_VIEWS.filter((meta) => getChildViews(meta.view).length > 0);

describe('links that carry filters', () => {
  // Every shape the dashboard itself produces: command palette, workload panel,
  // context banners, fleet, intelligence, cost and platform pages.
  it.each([
    ['/clusters?kind=Pod', '/clusters/browse?kind=Pod'],
    ['/clusters?kind=Node', '/clusters/browse?kind=Node'],
    ['/clusters?cluster=prod', '/clusters/browse?cluster=prod'],
    ['/clusters?cluster=prod&namespace=shop', '/clusters/browse?cluster=prod&namespace=shop'],
    [
      '/clusters?cluster=prod&namespace=shop&kind=Deployment&workload=shop%2Fweb',
      '/clusters/browse?cluster=prod&namespace=shop&kind=Deployment&workload=shop%2Fweb',
    ],
    ['/clusters?workload=web', '/clusters/browse?workload=web'],
    ['/clusters?tab=network', '/clusters/network?tab=network'],
    [
      '/clusters?cluster=prod&namespace=shop&kind=CiliumNetworkPolicy&tab=network',
      '/clusters/network?cluster=prod&namespace=shop&kind=CiliumNetworkPolicy&tab=network',
    ],
    ['/ai?tab=pipeline', '/ai/pipeline?tab=pipeline'],
    ['/ai?tab=analyze&workload=web', '/ai/analyze?tab=analyze&workload=web'],
    ['/ai?tab=optimize', '/ai/optimize?tab=optimize'],
    ['/ai?workload=web', '/ai/optimize?workload=web'],
    ['/affinity?workload=web', '/affinity/recommend?workload=web'],
    ['/affinity?tab=matrix', '/affinity/matrix?tab=matrix'],
    ['/alerts?workload=web', '/alerts/rules?workload=web'],
    ['/cost?workload=web', '/cost/estimate?workload=web'],
    ['/cost?tab=optimize', '/cost/intelligence?tab=optimize'],
    ['/fleet?workload=web', '/fleet/placement?workload=web'],
    ['/fleet?cluster=prod', '/fleet/overview?cluster=prod'],
    ['/fleet?tab=edge', '/fleet/edge?tab=edge'],
    ['/fleet?tab=placement&workload=web', '/fleet/placement?tab=placement&workload=web'],
    ['/gitops?workload=web', '/gitops/sync?workload=web'],
    ['/intelligence?workload=web&tab=predictions', '/intelligence/predictions?workload=web&tab=predictions'],
    ['/intelligence?tab=cost', '/intelligence/cost?tab=cost'],
    ['/intelligence?workload=web&tab=place', '/intelligence/placement?workload=web&tab=place'],
    ['/intelligence?tab=evolution', '/intelligence/evolution?tab=evolution'],
    ['/intelligence?workload=web', '/intelligence/predictions?workload=web'],
    ['/labs?tab=graph-search', '/labs/graph-platform?tab=graph-search'],
    ['/metrics?workload=web', '/metrics/prometheus?workload=web'],
    ['/metrics?q=http_requests', '/metrics/prometheus?q=http_requests'],
    ['/platform?workload=web', '/platform/runtime?workload=web'],
    ['/security?workload=web', '/security/overview?workload=web'],
  ])('%s opens %s', (from, to) => {
    expect(open(from)).toBe(to);
  });

  it('keeps the fragment and parameters it does not know', () => {
    expect(open('/platform?workload=web&ref=mail#trust')).toBe('/platform/runtime?workload=web&ref=mail#trust');
  });

  it('accepts a trailing slash', () => {
    expect(open('/clusters/?kind=Pod')).toBe('/clusters/browse?kind=Pod');
  });

  it('leaves hubs that show the workload themselves', () => {
    expect(open('/fabric?workload=web')).toBe('/fabric?workload=web');
    expect(open('/migrations?workload=web')).toBe('/migrations?workload=web');
  });
});

describe('hubs without filters', () => {
  it('covers all seventeen hubs', () => {
    expect(HUBS).toHaveLength(17);
  });

  it.each(HUBS.map((meta) => meta.path))('%s stays a hub', (path) => {
    expect(open(path)).toBe(path);
    expect(open(`${path}?workload=`)).toBe(`${path}?workload=`);
    expect(open(`${path}?utm_source=mail`)).toBe(`${path}?utm_source=mail`);
    expect(open(`${path}?tab=nonsense`)).toBe(`${path}?tab=nonsense`);
  });

  it('waits until sign-in has removed the token', () => {
    expect(open('/clusters?token=abc&kind=Pod')).toBe('/clusters?token=abc&kind=Pod');
  });

  it('can be asked for explicitly while keeping the filter', () => {
    expect(open('/platform?workload=web', SHOW_HUB_STATE)).toBe('/platform?workload=web');
  });
});

describe('redirects', () => {
  it('only ever leave a hub for one of its own sub-pages, so they cannot loop', () => {
    for (const [hub, filters] of Object.entries(HUB_FILTERS)) {
      for (const target of Object.values(filters ?? {})) {
        expect(getParentView(target)).toBe(hub);
      }
    }
    for (const meta of DASHBOARD_VIEWS) {
      const target = hubTarget(meta.view, '?workload=web&tab=network&cluster=c&q=x');
      if (target) expect(hubTarget(target, '?workload=web&tab=network&cluster=c&q=x')).toBeNull();
    }
  });

  it('never touch a page that is not a hub', () => {
    const others: AppView[] = ['workloads', 'events', 'zyra', 'clusters-browse'];
    for (const view of others) expect(hubTarget(view, '?workload=web&tab=network')).toBeNull();
  });
});

function markup(address: string, element: ReturnType<typeof createElement>): string {
  return renderToStaticMarkup(createElement(MemoryRouter, { initialEntries: [address] }, element));
}

describe('moving around a section', () => {
  const links = [{ view: 'clusters-browse' as AppView, title: 'Browse', description: 'd', icon: null }];

  it('hub bands carry the workload in focus', () => {
    const html = markup('/clusters?workload=web', createElement(SectionHubPage, { links }));
    expect(html).toContain('href="/clusters/browse?workload=web"');
  });

  it('hub bands stay plain without one', () => {
    const html = markup('/clusters', createElement(SectionHubPage, { links }));
    expect(html).toContain('href="/clusters/browse"');
  });

  it('the Summary tab keeps the workload', () => {
    const html = markup('/clusters/browse?workload=web', createElement(SectionTabs, { currentView: 'clusters-browse' }));
    expect(html).toContain('href="/clusters?workload=web"');
  });
});
