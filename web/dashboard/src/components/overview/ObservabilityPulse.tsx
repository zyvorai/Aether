// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router';
import { apiFetchSettled } from '../../utils/api';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery } from '../../utils/urlState';
import type { EventSummary, HealthSummary } from '../../types/api';
import StatRibbon from '../StatRibbon';

type Loaded<T> = { state: 'loading' } | { state: 'failed' } | { state: 'ready'; data: T };

function useSummary<T>(path: string, refreshKey?: number): Loaded<T> {
  const [value, setValue] = useState<Loaded<T>>({ state: 'loading' });
  useEffect(() => {
    let cancelled = false;
    void apiFetchSettled<T>(path).then((result) => {
      if (!cancelled) setValue(result.ok ? { state: 'ready', data: result.data } : { state: 'failed' });
    });
    return () => {
      cancelled = true;
    };
  }, [path, refreshKey]);
  return value;
}

/** The live numbers behind the hub's destinations. A number that did not load is a dash, never a zero. */
export default function ObservabilityPulse({ refreshKey }: { refreshKey?: number }) {
  const navigate = useNavigate();
  const health = useSummary<HealthSummary>('/orchestrator/summary', refreshKey);
  const events = useSummary<EventSummary>('/events/summary', refreshKey);

  const h = health.state === 'ready' ? health.data : null;
  const e = events.state === 'ready' ? events.data : null;
  const monitored = h ? h.healthy + h.degraded + h.unhealthy + h.unknown : null;
  const missing = [health.state === 'failed' ? 'health' : null, events.state === 'failed' ? 'events' : null].filter(Boolean);
  const go = (view: 'health' | 'events', query: Record<string, string> = {}) => navigate(pathWithQuery(viewToPath(view), query));

  return (
    <section aria-label="Live summary" data-testid="observability-pulse">
      <StatRibbon
        items={[
          { label: 'Healthy', value: h?.healthy ?? '—', tone: 'emerald', onClick: () => go('health', { status: 'healthy' }) },
          { label: 'Degraded', value: h?.degraded ?? '—', tone: 'amber', onClick: () => go('health', { status: 'degraded' }) },
          { label: 'Unhealthy', value: h?.unhealthy ?? '—', tone: 'red', onClick: () => go('health', { status: 'unhealthy' }) },
          { label: 'Unacknowledged events', value: e?.unacknowledged ?? '—', onClick: () => go('events', { ack: 'open' }) },
          { label: 'Critical open', value: e?.critical_unacked ?? '—', tone: 'red', onClick: () => go('events', { severity: 'critical' }) },
        ]}
      />
      {missing.length > 0 ? (
        <p className="mt-2 text-sm text-warning" role="status">
          Could not load the {missing.join(' and ')} summary.
        </p>
      ) : monitored === 0 ? (
        <p className="mt-2 text-sm text-muted">The health orchestrator is not monitoring any workloads.</p>
      ) : null}
    </section>
  );
}
