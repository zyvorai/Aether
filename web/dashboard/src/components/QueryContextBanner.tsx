// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import type { ReactNode } from 'react';
import { Link, useNavigate } from 'react-router';
import { viewToPath } from '../utils/dashboardRoutes';
import { pathWithQuery } from '../utils/urlState';

export type WorkloadCrossLinkOptions = {
  workload: string;
  prefix: string;
  eventsCategory?: string;
  showDrift?: boolean;
  showAudit?: boolean;
  showGitops?: boolean;
  showMetrics?: boolean;
};

/** Scoped cross-links for workload context banners (Events / Alerts / Health / Trust + optional ops links). */
export function WorkloadScopedCrossLinks({
  workload,
  prefix,
  eventsCategory,
  showDrift = false,
  showAudit = false,
  showGitops = false,
  showMetrics = false,
}: WorkloadCrossLinkOptions) {
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
      {showDrift ? (
        <>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('drift'), { workload: name })}
            className="text-aether hover:underline"
            data-testid={`${prefix}-drift-link`}
          >
            Drift →
          </Link>
        </>
      ) : null}
      {showAudit ? (
        <>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('audit'), { workload: name })}
            className="text-aether hover:underline"
            data-testid={`${prefix}-audit-link`}
          >
            Audit →
          </Link>
        </>
      ) : null}
      {showGitops ? (
        <>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('gitops'), { workload: name })}
            className="text-aether hover:underline"
            data-testid={`${prefix}-gitops-link`}
          >
            GitOps →
          </Link>
        </>
      ) : null}
      {showMetrics ? (
        <>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('metrics'), { workload: name })}
            className="text-aether hover:underline"
            data-testid={`${prefix}-metrics-link`}
          >
            Metrics →
          </Link>
        </>
      ) : null}
    </>
  );
}

export function WorkloadContextBanner({
  testId,
  workload,
  description,
  openTestId,
  children,
}: {
  testId: string;
  workload: string;
  description?: string;
  openTestId?: string;
  children?: ReactNode;
}) {
  const navigate = useNavigate();
  const name = workload.trim();
  if (!name) return null;

  return (
    <div
      data-testid={testId}
      className="mb-6 glass-context-banner"
    >
      {description ?? 'Workload context'} for <span className="font-mono text-aether">{name}</span>
      {' · '}
      <button
        type="button"
        onClick={() => navigate(pathWithQuery(viewToPath('workloads'), { workload: name }))}
        className="text-aether hover:underline"
        data-testid={openTestId ?? `${testId}-open`}
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
      className="mb-6 glass-context-banner"
    >
      Filtered {entityLabel} matching <span className="font-mono text-aether">{q}</span>
      {' · '}
      <button
        type="button"
        onClick={() => navigate(pathWithQuery(viewToPath('workloads'), { workload: q }))}
        className="text-aether hover:underline"
        data-testid={`${testId}-open`}
      >
        Open workload →
      </button>
      {children}
    </div>
  );
}
