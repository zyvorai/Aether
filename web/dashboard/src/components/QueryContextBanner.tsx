// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import type { ReactNode } from 'react';
import { Link, useNavigate } from 'react-router';
import { viewToPath } from '../utils/dashboardRoutes';
import { pathWithQuery } from '../utils/urlState';

/** Scoped events / alerts / health / trust links for workload context banners. */
export function WorkloadScopedCrossLinks({
  workload,
  prefix,
  eventsCategory,
}: {
  workload: string;
  prefix: string;
  eventsCategory?: string;
}) {
  const name = workload.trim();
  if (!name) return null;

  const eventsQuery = eventsCategory
    ? { workload: name, category: eventsCategory }
    : { workload: name };

  return (
    <>
      {' · '}
      <Link
        to={pathWithQuery(viewToPath('events'), eventsQuery)}
        className="text-aether hover:underline"
        data-testid={`${prefix}-events-link`}
      >
        Events →
      </Link>
      {' · '}
      <Link
        to={pathWithQuery(viewToPath('alerts'), { workload: name })}
        className="text-aether hover:underline"
        data-testid={`${prefix}-alerts-link`}
      >
        Alerts →
      </Link>
      {' · '}
      <Link
        to={pathWithQuery(viewToPath('health'), { workload: name })}
        className="text-aether hover:underline"
        data-testid={`${prefix}-health-link`}
      >
        Health →
      </Link>
      {' · '}
      <Link
        to={pathWithQuery(viewToPath('workloads'), { workload: name, tab: 'trust' })}
        className="text-aether hover:underline"
        data-testid={`${prefix}-trust-link`}
      >
        Trust →
      </Link>
    </>
  );
}

export function WorkloadContextBanner({
  testId,
  workload,
  description,
  children,
}: {
  testId: string;
  workload: string;
  description?: string;
  children?: ReactNode;
}) {
  const navigate = useNavigate();
  const name = workload.trim();
  if (!name) return null;

  return (
    <div
      data-testid={testId}
      className="mb-6 rounded-xl border border-aether/30 bg-aether/5 px-4 py-3 text-sm text-slate-300"
    >
      {description ?? 'Workload context'} for <span className="font-mono text-aether">{name}</span>
      {' · '}
      <button
        type="button"
        onClick={() => navigate(pathWithQuery(viewToPath('workloads'), { workload: name }))}
        className="text-aether hover:underline"
      >
        Open workload →
      </button>
      {children}
    </div>
  );
}

export function SearchQueryContextBanner({
  testId,
  query,
  entityLabel,
  children,
}: {
  testId: string;
  query: string;
  entityLabel: string;
  children?: ReactNode;
}) {
  const navigate = useNavigate();
  const q = query.trim();
  if (!q) return null;

  return (
    <div
      data-testid={testId}
      className="mb-6 rounded-xl border border-aether/30 bg-aether/5 px-4 py-3 text-sm text-slate-300"
    >
      Filtered {entityLabel} matching <span className="font-mono text-aether">{q}</span>
      {' · '}
      <button
        type="button"
        onClick={() => navigate(pathWithQuery(viewToPath('workloads'), { workload: q }))}
        className="text-aether hover:underline"
      >
        Open workload →
      </button>
      {children}
    </div>
  );
}
