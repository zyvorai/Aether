// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import { useState, useEffect, useCallback, useMemo } from 'react';
import { Link, useSearchParams } from 'react-router';
import { Inbox } from 'lucide-react';
import { apiFetchSettled } from '../../utils/api';
import { formatTimestamp } from '../../utils/formatters';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery, useQueryParam } from '../../utils/urlState';
import { useBufferedValue } from '../../hooks/useBufferedValue';
import PageToolbar from '../PageToolbar';
import Badge, { SeverityBadge } from '../Badge';
import EmptyState from '../EmptyState';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import StatRibbon from '../StatRibbon';
import { clockTime, groupByDay } from '../../utils/eventGroups';
import { WorkloadContextBanner, WorkloadScopedCrossLinks } from '../QueryContextBanner';
import type { Event, EventSummary } from '../../types/api';

const PAGE_SIZE = 100;

function EventsPage({ refreshKey }: { refreshKey?: number } = {}) {
  const [events, setEvents] = useState<Event[]>([]);
  const [summary, setSummary] = useState<EventSummary | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [loadError, setLoadError] = useState<string | undefined>(undefined);
  const [hasLoadedOnce, setHasLoadedOnce] = useState(false);
  const [search, setSearch] = useQueryParam('q');
  const [severity, setSeverity] = useQueryParam('severity', 'all');
  const [ack, setAck] = useQueryParam('ack', 'all');
  const [limit, setLimit] = useState(PAGE_SIZE);
  const [category, setCategory] = useQueryParam('category', 'all');
  const [workloadFilter, setWorkloadFilter] = useQueryParam('workload');
  const [bufferedWorkloadFilter, setBufferedWorkloadFilter] = useBufferedValue(workloadFilter, setWorkloadFilter);
  const [, setSearchParams] = useSearchParams();

  const clearFilters = () => {
    setSearchParams(
      (prev) => {
        const copy = new URLSearchParams(prev);
        copy.delete('q');
        copy.delete('severity');
        copy.delete('ack');
        copy.delete('category');
        copy.delete('workload');
        return copy;
      },
      { replace: true },
    );
  };

  const load = useCallback(async () => {
    setLoading(true);
    setLoadFailed(false);
    const qs = new URLSearchParams();
    if (category && category !== 'all') qs.set('category', category);
    if (workloadFilter.trim()) qs.set('workload', workloadFilter.trim());
    const query = qs.toString() ? `?${qs}` : '';
    const [ev, s] = await Promise.all([
      apiFetchSettled<Event[]>(`/events${query}`),
      apiFetchSettled<EventSummary>('/events/summary'),
    ]);
    // An empty list must mean "no events", never "the request failed".
    if (!ev.ok) {
      setLoadFailed(true);
      setLoadError(ev.detail);
      setEvents([]);
      setSummary(null);
    } else {
      setEvents(ev.data);
      setSummary(s.ok ? s.data : null);
    }
    setLoading(false);
    setHasLoadedOnce(true);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- refreshKey intentionally triggers a refetch in place, not a remount.
  }, [category, workloadFilter, refreshKey]);

  useEffect(() => {
    void load();
  }, [load]);

  const severityOptions = useMemo(() => {
    const levels = new Set(events.map((e) => e.severity.toLowerCase()));
    return ['all', ...Array.from(levels).sort()];
  }, [events]);

  const filtered = useMemo(() => {
    const q = search.trim().toLowerCase();
    return events.filter((ev) => {
      if (severity !== 'all' && ev.severity.toLowerCase() !== severity) return false;
      if (ack === 'open' && ev.acknowledged) return false;
      if (!q) return true;
      return (
        ev.message.toLowerCase().includes(q) ||
        ev.title.toLowerCase().includes(q) ||
        (ev.workload?.toLowerCase().includes(q) ?? false)
      );
    });
  }, [events, search, severity, ack]);

  if (loading && !hasLoadedOnce && !loadFailed) {
    return <PageLoading rows={5} />;
  }

  if (loadFailed) {
    return <PageLoadError title="Events unavailable" detail={loadError} onRetry={() => void load()} />;
  }

  return (
    <div>
      <WorkloadContextBanner
        testId="events-workload-context"
        workload={workloadFilter}
        description="Events for workload"
      >
        <WorkloadScopedCrossLinks
          workload={workloadFilter}
          prefix="events"
          eventsCategory={category !== 'all' ? category : undefined}
          showDrift
          showAudit
          showGitops
          showMetrics
        />
        {workloadFilter.trim() ? (
          <>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('openapi'), { workload: workloadFilter.trim() })}
              className="text-primary hover:underline"
              data-testid="events-openapi-link"
            >
              OpenAPI →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('audit'), { workload: workloadFilter.trim() })}
              className="text-primary hover:underline"
              data-testid="events-audit-link"
            >
              Audit →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('zyra'), { workload: workloadFilter.trim(), q: `Explain recent events for ${workloadFilter.trim()}` })}
              className="text-primary hover:underline"
              data-testid="events-copilot-link"
            >
              Copilot →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('secrets'), { workload: workloadFilter.trim() })}
              className="text-primary hover:underline"
              data-testid="events-context-secrets-link"
            >
              Secrets →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('policy'), { workload: workloadFilter.trim() })}
              className="text-primary hover:underline"
              data-testid="events-context-policy-link"
            >
              Policy →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('editor'), { workload: workloadFilter.trim() })}
              className="text-primary hover:underline"
              data-testid="events-context-editor-link"
            >
              Editor →
            </Link>
          </>
        ) : null}
      </WorkloadContextBanner>
      {summary && (
        <div className="flex flex-wrap items-end gap-3">
          <StatRibbon
            items={[
              { label: 'Total', value: summary.total_events, onClick: clearFilters, testId: 'events-total-stat' },
              { label: 'Unacknowledged', value: summary.unacknowledged, onClick: () => setAck(ack === 'open' ? 'all' : 'open'), active: ack === 'open', testId: 'events-unacked-stat' },
              { label: 'Critical open', value: summary.critical_unacked, tone: summary.critical_unacked > 0 ? 'red' : 'white', onClick: () => setSeverity(severity === 'critical' ? 'all' : 'critical'), active: severity === 'critical', testId: 'events-critical-stat' },
            ]}
          />
          <Link
            to={
              workloadFilter.trim()
                ? pathWithQuery(viewToPath('alerts'), { workload: workloadFilter.trim() })
                : viewToPath('alerts')
            }
            className="mb-1 text-xs text-primary hover:underline"
            data-testid="events-alerts-link"
          >
            Alert channels →
          </Link>
        </div>
      )}

      <PageToolbar
        search={search}
        onSearchChange={setSearch}
        searchPlaceholder="Search message or workload…"
        searchTestId="events-search"
        refreshTestId="events-refresh"
        onRefresh={() => void load()}
        refreshing={loading}
        filters={
          <>
            <select
              value={category}
              onChange={(e) => setCategory(e.target.value)}
              className="glass-select"
              style={{ width: 'auto', minWidth: '9rem' }}
              aria-label="Category filter"
              data-testid="events-category-filter"
            >
              <option value="all">All categories</option>
              <option value="intent-violation">Intent violations</option>
              <option value="drift">Drift</option>
              <option value="policy">Policy</option>
              <option value="sla">SLA</option>
              <option value="health">Health</option>
            </select>
            {category === 'drift' && (
              <Link to={viewToPath('drift')} className="text-xs text-primary hover:underline self-center">
                Open drift page →
              </Link>
            )}
            {category === 'policy' && (
              <Link to={viewToPath('policy')} className="text-xs text-primary hover:underline self-center" data-testid="events-policy-link">
                Open policy check →
              </Link>
            )}
            {category === 'intent-violation' && (
              <Link
                to={viewToPath('ai')}
                className="text-xs text-primary hover:underline self-center"
                data-testid="events-intent-link"
              >
                AI intent debugger →
              </Link>
            )}
            {category === 'sla' && (
              <Link to={viewToPath('sla')} className="text-xs text-primary hover:underline self-center" data-testid="events-sla-link">
                SLA compliance →
              </Link>
            )}
            {category === 'health' && (
              <Link
                to={viewToPath('health')}
                className="text-xs text-primary hover:underline self-center"
                data-testid="events-health-link"
              >
                Health monitor →
              </Link>
            )}
            <input
              type="text"
              value={bufferedWorkloadFilter}
              onChange={(e) => setBufferedWorkloadFilter(e.target.value)}
              placeholder="Workload name…"
              className="glass-select"
              style={{ width: 'auto', minWidth: '10rem' }}
              aria-label="Workload filter"
              data-testid="events-workload-filter"
            />
            <select
              value={severity}
              onChange={(e) => setSeverity(e.target.value)}
              className="glass-select"
              style={{ width: 'auto', minWidth: '9rem' }}
              aria-label="Severity filter"
              data-testid="events-severity-filter"
            >
              {severityOptions.map((opt) => (
                <option key={opt} value={opt}>
                  {opt === 'all' ? 'All severities' : opt}
                </option>
              ))}
            </select>
            {(search || severity !== 'all' || ack !== 'all' || category !== 'all' || workloadFilter.trim()) && (
              <button
                type="button"
                data-testid="events-clear-filters"
                onClick={clearFilters}
                className="rounded-xl border glass-divider px-3 py-2 text-xs text-muted hover:text-primary hover:border-primary/40"
              >
                Clear filters
              </button>
            )}
          </>
        }
      />

      {events.length === 0 ? (
        <EmptyState icon={<Inbox size={48} />} title="No events" description="No events have been recorded" />
      ) : filtered.length === 0 ? (
        <EmptyState icon={<Inbox size={48} />} title="No matching events" description="Try adjusting search or severity filter" />
      ) : (
        <div className="glass event-list" data-testid="events-list">
          {groupByDay(filtered.slice(0, limit)).map((group) => (
            <section key={group.day} aria-label={group.label}>
              <h2 className="event-day">{group.label}</h2>
              <ul>
                {group.items.map((ev, i) => (
                  <li key={`${ev.timestamp}-${i}`} className="event-row">
                    <span className="event-severity"><SeverityBadge severity={ev.severity} /></span>
                    <div className="min-w-0 flex-1">
                      <div className="text-sm font-medium text-foreground">{ev.title}</div>
                      <div className="break-words text-xs text-muted">{ev.message}</div>
                      <div className="mt-1 flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-subtle">
                        <Badge text={ev.category} variant="muted" />
                        {ev.workload ? (
                          <Link to={pathWithQuery(viewToPath('workloads'), { workload: ev.workload })} className="text-primary hover:underline">
                            {ev.workload}
                          </Link>
                        ) : null}
                        {ev.workload && ev.category === 'drift' ? (
                          <Link to={viewToPath('drift')} className="text-primary hover:underline">
                            Open drift page
                          </Link>
                        ) : null}
                        <span>Source: {ev.source}</span>
                        {ev.acknowledged && <Badge text="ACK" variant="green" />}
                      </div>
                    </div>
                    <time className="event-time" dateTime={ev.timestamp} title={formatTimestamp(ev.timestamp)}>
                      {clockTime(ev.timestamp)}
                    </time>
                  </li>
                ))}
              </ul>
            </section>
          ))}
          {filtered.length > limit ? (
            <button type="button" className="mt-2 text-sm text-primary hover:underline" onClick={() => setLimit((n) => n + PAGE_SIZE)}>
              Show more ({filtered.length - limit} left)
            </button>
          ) : null}
        </div>
      )}
    </div>
  );
}

export default withAuroraPage('events', EventsPage);
