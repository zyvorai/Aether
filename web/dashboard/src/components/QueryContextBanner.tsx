// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { useState, type ReactNode } from 'react';
import { Link, useNavigate } from 'react-router';
import { ChevronDown, ChevronRight } from 'lucide-react';
import { viewToPath } from '../utils/dashboardRoutes';
import { pathWithQuery } from '../utils/urlState';
import type { AppView } from '../types/api';

export interface CrossLink {
  label: string;
  view: AppView;
  testId: string;
  query?: Record<string, string>;
}

export type WorkloadCrossLinkOptions = {
  workload: string;
  prefix: string;
  eventsCategory?: string;
  showDrift?: boolean;
  showAudit?: boolean;
  showGitops?: boolean;
  showMetrics?: boolean;
  /** Additional page-specific links beyond the standard Events/Alerts/Health/Trust set — collapsed behind a "+N more" toggle once the total exceeds the visible cap (see DESIGN.md: "collapse ... quick-link chip walls ... (≤8 links)"). */
  extraLinks?: CrossLink[];
};

const VISIBLE_LINK_CAP = 5;

/** Scoped cross-links for workload context banners (Events / Alerts / Health / Trust + optional ops/extra links), collapsing beyond DESIGN.md's ≤8-link chip-wall cap. */
export function WorkloadScopedCrossLinks({
  workload,
  prefix,
  eventsCategory,
  showDrift = false,
  showAudit = false,
  showGitops = false,
  showMetrics = false,
  extraLinks = [],
}: WorkloadCrossLinkOptions) {
  const [expanded, setExpanded] = useState(false);
  const name = workload.trim();
  if (!name) return null;

  const eventsQuery: Record<string, string> = eventsCategory
    ? { workload: name, category: eventsCategory }
    : { workload: name };

  const links: CrossLink[] = [
    { label: 'Events →', view: 'events', testId: `${prefix}-events-link`, query: eventsQuery },
    { label: 'Alerts →', view: 'alerts', testId: `${prefix}-alerts-link`, query: { workload: name } },
    { label: 'Health →', view: 'health', testId: `${prefix}-health-link`, query: { workload: name } },
    { label: 'Trust →', view: 'workloads', testId: `${prefix}-trust-link`, query: { workload: name, tab: 'trust' } },
    ...(showDrift ? [{ label: 'Drift →', view: 'drift' as AppView, testId: `${prefix}-drift-link`, query: { workload: name } }] : []),
    ...(showAudit ? [{ label: 'Audit →', view: 'audit' as AppView, testId: `${prefix}-audit-link`, query: { workload: name } }] : []),
    ...(showGitops ? [{ label: 'GitOps →', view: 'gitops' as AppView, testId: `${prefix}-gitops-link`, query: { workload: name } }] : []),
    ...(showMetrics ? [{ label: 'Metrics →', view: 'metrics' as AppView, testId: `${prefix}-metrics-link`, query: { workload: name } }] : []),
    ...extraLinks,
  ];

  const visible = expanded ? links : links.slice(0, VISIBLE_LINK_CAP);
  const hidden = links.length - visible.length;

  return (
    <>
      {visible.map((link) => (
        <span key={link.testId}>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath(link.view), link.query ?? { workload: name })}
            className="text-primary hover:underline"
            data-testid={link.testId}
          >
            {link.label}
          </Link>
        </span>
      ))}
      {hidden > 0 ? (
        <>
          {' · '}
          <button
            type="button"
            onClick={() => setExpanded(true)}
            className="inline-flex items-center gap-0.5 text-primary hover:underline"
            data-testid={`${prefix}-links-more`}
          >
            +{hidden} more <ChevronRight className="h-3 w-3" />
          </button>
        </>
      ) : expanded && links.length > VISIBLE_LINK_CAP ? (
        <>
          {' · '}
          <button
            type="button"
            onClick={() => setExpanded(false)}
            className="inline-flex items-center gap-0.5 text-primary hover:underline"
            data-testid={`${prefix}-links-less`}
          >
            Show fewer <ChevronDown className="h-3 w-3" />
          </button>
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
      {description ?? 'Workload context'} for <span className="font-mono text-primary">{name}</span>
      {' · '}
      <button
        type="button"
        onClick={() => navigate(pathWithQuery(viewToPath('workloads'), { workload: name }))}
        className="text-primary hover:underline"
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
      Filtered {entityLabel} matching <span className="font-mono text-primary">{q}</span>
      {' · '}
      <button
        type="button"
        onClick={() => navigate(pathWithQuery(viewToPath('workloads'), { workload: q }))}
        className="text-primary hover:underline"
        data-testid={`${testId}-open`}
      >
        Open workload →
      </button>
      {children}
    </div>
  );
}
