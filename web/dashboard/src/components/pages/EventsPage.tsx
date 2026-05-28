// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect, useCallback, useMemo } from 'react';
import { Link, useSearchParams } from 'react-router';
import { Inbox } from 'lucide-react';
import { apiFetchSettled } from '../../utils/api';
import { formatTimestamp } from '../../utils/formatters';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery, useQueryParam } from '../../utils/urlState';
import PageToolbar from '../PageToolbar';
import StatCard from '../StatCard';
import Badge, { SeverityBadge } from '../Badge';
import EmptyState from '../EmptyState';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import { WorkloadContextBanner, WorkloadScopedCrossLinks } from '../QueryContextBanner';
import type { Event, EventSummary } from '../../types/api';

export default function EventsPage() {
  const [events, setEvents] = useState<Event[]>([]);
  const [summary, setSummary] = useState<EventSummary | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [search, setSearch] = useQueryParam('q');
  const [severity, setSeverity] = useQueryParam('severity', 'all');
  const [category, setCategory] = useQueryParam('category', 'all');
  const [workloadFilter, setWorkloadFilter] = useQueryParam('workload');
  const [, setSearchParams] = useSearchParams();

  const clearFilters = () => {
    setSearchParams(
      (prev) => {
        const copy = new URLSearchParams(prev);
        copy.delete('q');
        copy.delete('severity');
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
    if (!ev.ok && !s.ok) {
      setLoadFailed(true);
      setEvents([]);
      setSummary(null);
    } else {
      setEvents(ev.ok ? ev.data : []);
      setSummary(s.ok ? s.data : null);
    }
    setLoading(false);
  }, [category, workloadFilter]);

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
      if (!q) return true;
      return (
        ev.message.toLowerCase().includes(q) ||
        ev.title.toLowerCase().includes(q) ||
        (ev.workload?.toLowerCase().includes(q) ?? false)
      );
    });
  }, [events, search, severity]);

  if (loading && events.length === 0 && !loadFailed) {
    return <PageLoading rows={5} />;
  }

  if (loadFailed) {
    return <PageLoadError title="Events unavailable" onRetry={() => void load()} />;
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
              className="text-aether hover:underline"
              data-testid="events-openapi-link"
            >
              OpenAPI →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('audit'), { workload: workloadFilter.trim() })}
              className="text-aether hover:underline"
              data-testid="events-audit-link"
            >
              Audit →
            </Link>
          </>
        ) : null}
      </WorkloadContextBanner>
      {summary && (
        <div className="grid grid-cols-2 lg:grid-cols-3 xl:grid-cols-6 gap-4 mb-6">
          <button type="button" onClick={() => setCategory('all')} className="text-left">
            <StatCard title="Total" value={summary.total_events} color="blue" />
          </button>
          <button type="button" onClick={() => setSeverity('warning')} className="text-left">
            <StatCard title="Unacknowledged" value={summary.unacknowledged} color="yellow" />
          </button>
          <button type="button" onClick={() => setSeverity('critical')} className="text-left">
            <StatCard title="Critical" value={summary.critical_unacked} color="red" />
          </button>
          <Link
            to={
              workloadFilter.trim()
                ? pathWithQuery(viewToPath('alerts'), { workload: workloadFilter.trim() })
                : viewToPath('alerts')
            }
            className="text-xs text-aether hover:underline self-end mb-1"
            data-testid="events-alerts-link"
          >
            Alert channels →
          </Link>
          {workloadFilter.trim() ? (
            <Link
              to={pathWithQuery(viewToPath('workloads'), {
                workload: workloadFilter.trim(),
                tab: 'trust',
              })}
              className="text-xs text-aether hover:underline self-end mb-1 ml-3"
              data-testid="events-trust-link"
            >
              Trust &amp; attestation →
            </Link>
          ) : null}
        </div>
      )}

      <PageToolbar
        search={search}
        onSearchChange={setSearch}
        searchPlaceholder="Search message or workload…"
        onRefresh={() => void load()}
        refreshing={loading}
        filters={
          <>
            <select
              value={category}
              onChange={(e) => setCategory(e.target.value)}
              className="rounded-xl border border-slate-700/80 bg-slate-950/60 px-3 py-2 text-sm text-slate-200"
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
              <Link to={viewToPath('drift')} className="text-xs text-aether hover:underline self-center">
                Open drift page →
              </Link>
            )}
            {category === 'policy' && (
              <Link to={viewToPath('policy')} className="text-xs text-aether hover:underline self-center" data-testid="events-policy-link">
                Open policy check →
              </Link>
            )}
            {category === 'intent-violation' && (
              <Link
                to={viewToPath('ai')}
                className="text-xs text-aether hover:underline self-center"
                data-testid="events-intent-link"
              >
                AI intent debugger →
              </Link>
            )}
            {category === 'sla' && (
              <Link to={viewToPath('sla')} className="text-xs text-aether hover:underline self-center" data-testid="events-sla-link">
                SLA compliance →
              </Link>
            )}
            {category === 'health' && (
              <Link
                to={viewToPath('health')}
                className="text-xs text-aether hover:underline self-center"
                data-testid="events-health-link"
              >
                Health monitor →
              </Link>
            )}
            <input
              type="text"
              value={workloadFilter}
              onChange={(e) => setWorkloadFilter(e.target.value)}
              placeholder="Workload name…"
              className="rounded-xl border border-slate-700/80 bg-slate-950/60 px-3 py-2 text-sm text-slate-200 min-w-[10rem]"
              aria-label="Workload filter"
              data-testid="events-workload-filter"
            />
            <select
              value={severity}
              onChange={(e) => setSeverity(e.target.value)}
              className="rounded-xl border border-slate-700/80 bg-slate-950/60 px-3 py-2 text-sm text-slate-200"
              aria-label="Severity filter"
              data-testid="events-severity-filter"
            >
              {severityOptions.map((opt) => (
                <option key={opt} value={opt}>
                  {opt === 'all' ? 'All severities' : opt}
                </option>
              ))}
            </select>
            {(search || severity !== 'all' || category !== 'all' || workloadFilter.trim()) && (
              <button
                type="button"
                data-testid="events-clear-filters"
                onClick={clearFilters}
                className="rounded-xl border border-slate-700 px-3 py-2 text-xs text-slate-400 hover:text-aether hover:border-aether/40"
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
        <div className="dash-card" data-testid="events-list">
          <div className="space-y-3 max-h-[600px] overflow-auto">
            {filtered.map((ev, i) => (
              <div key={`${ev.timestamp}-${i}`} className="flex items-start gap-3 p-4 bg-slate-950/50 rounded-xl border border-slate-800/50">
                <div className="flex flex-col gap-1.5 shrink-0">
                  <SeverityBadge severity={ev.severity} />
                  <Badge text={ev.category} variant="muted" />
                </div>
                <div className="flex-1 min-w-0">
                  <div className="text-sm font-medium text-slate-200">{ev.title}</div>
                  <div className="text-xs text-slate-400 mt-1">{ev.message}</div>
                  <div className="flex items-center gap-3 mt-2 text-xs text-slate-500 flex-wrap">
                    <span>{formatTimestamp(ev.timestamp)}</span>
                    {ev.workload ? (
                      <span>
                        Workload:{' '}
                        <Link
                          to={pathWithQuery(viewToPath('workloads'), { workload: ev.workload })}
                          className="text-aether hover:underline"
                        >
                          {ev.workload}
                        </Link>
                      </span>
                    ) : null}
                    {ev.workload && ev.category === 'drift' ? (
                      <Link to={viewToPath('drift')} className="text-aether hover:underline">
                        Open drift page
                      </Link>
                    ) : null}
                    <span>Source: {ev.source}</span>
                    {ev.acknowledged && <Badge text="ACK" variant="green" />}
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
