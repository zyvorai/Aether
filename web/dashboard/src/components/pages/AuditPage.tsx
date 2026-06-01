// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect, useCallback, useMemo } from 'react';
import { Link } from 'react-router';
import { Download, Inbox, ShieldCheck } from 'lucide-react';
import { apiFetch, apiFetchSettled } from '../../utils/api';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery } from '../../utils/urlState';
import { useQueryParam } from '../../utils/urlState';
import { formatTimestamp } from '../../utils/formatters';
import PageToolbar from '../PageToolbar';
import StatCard from '../StatCard';
import Badge from '../Badge';
import EmptyState from '../EmptyState';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import { WorkloadContextBanner, WorkloadScopedCrossLinks } from '../QueryContextBanner';
import type { AuditResponse, AuditVerifyResponse } from '../../types/api';

export default function AuditPage() {
  const [audit, setAudit] = useState<AuditResponse | null>(null);
  const [verify, setVerify] = useState<AuditVerifyResponse | null>(null);
  const [verifying, setVerifying] = useState(false);
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [search, setSearch] = useQueryParam('q');
  const [workloadFilter, setWorkloadFilter] = useQueryParam('workload');
  const [resultFilter, setResultFilter] = useQueryParam('result');

  const load = useCallback(async () => {
    setLoading(true);
    setLoadFailed(false);
    const [data, verifyData] = await Promise.all([
      apiFetchSettled<AuditResponse>('/audit'),
      apiFetchSettled<AuditVerifyResponse>('/audit/verify'),
    ]);
    if (!data.ok && !verifyData.ok) {
      setLoadFailed(true);
      setAudit(null);
      setVerify(null);
    } else {
      setAudit(data.ok ? data.data : null);
      setVerify(verifyData.ok ? verifyData.data : null);
    }
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function reVerify() {
    setVerifying(true);
    const result = await apiFetchSettled<AuditVerifyResponse>('/audit/verify');
    if (result.ok) setVerify(result.data);
    setVerifying(false);
  }

  const filteredEvents = useMemo(() => {
    if (!audit) return [];
    const q = search.trim().toLowerCase();
    const wl = workloadFilter.trim().toLowerCase();
    return audit.recent_events.filter((ev) => {
      if (wl && !ev.workload.toLowerCase().includes(wl)) return false;
      if (resultFilter === 'success' && ev.result.toLowerCase() !== 'success') return false;
      if (resultFilter === 'failure' && ev.result.toLowerCase() === 'success') return false;
      if (!q) return true;
      return (
        ev.action.toLowerCase().includes(q) ||
        ev.workload.toLowerCase().includes(q) ||
        ev.message.toLowerCase().includes(q)
      );
    });
  }, [audit, search, workloadFilter, resultFilter]);

  function downloadExport(format: 'json' | 'csv') {
    const rows = filteredEvents;
    if (rows.length === 0) return;
    let body: string;
    let mime: string;
    let filename: string;
    if (format === 'json') {
      body = JSON.stringify(rows, null, 2);
      mime = 'application/json';
      filename = 'aether-audit-export.json';
    } else {
      const header = 'id,timestamp,action,workload,result,runtime,message';
      const lines = rows.map(
        (ev) =>
          `"${ev.id}","${ev.timestamp}","${ev.action.replace(/"/g, '""')}","${ev.workload.replace(/"/g, '""')}","${ev.result}","${(ev.runtime ?? '').replace(/"/g, '""')}","${ev.message.replace(/"/g, '""')}"`,
      );
      body = [header, ...lines].join('\n');
      mime = 'text/csv';
      filename = 'aether-audit-export.csv';
    }
    const blob = new Blob([body], { type: mime });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    a.click();
    URL.revokeObjectURL(url);
  }

  if (loading && !audit && !loadFailed) {
    return <PageLoading rows={5} />;
  }

  if (loadFailed) {
    return <PageLoadError title="Audit trail unavailable" onRetry={() => void load()} />;
  }

  if (!audit) {
    return (
      <EmptyState icon={<Inbox size={48} />} title="No audit data" description="No audit events have been recorded" />
    );
  }

  return (
    <div>
      <WorkloadContextBanner
        testId="audit-workload-context"
        workload={workloadFilter}
        description="Audit entries for workload"
      >
        <WorkloadScopedCrossLinks workload={workloadFilter} prefix="audit" showGitops showDrift />
        {workloadFilter.trim() ? (
          <>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('openapi'), { workload: workloadFilter.trim() })}
              className="text-aether hover:underline"
              data-testid="audit-openapi-link"
            >
              OpenAPI →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('rbac'), { workload: workloadFilter.trim() })}
              className="text-aether hover:underline"
              data-testid="audit-rbac-link"
            >
              RBAC →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('secrets'), { workload: workloadFilter.trim() })}
              className="text-aether hover:underline"
              data-testid="audit-secrets-link"
            >
              Secrets →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('policy'), { workload: workloadFilter.trim() })}
              className="text-aether hover:underline"
              data-testid="audit-context-policy-link"
            >
              Policy →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('editor'), { workload: workloadFilter.trim() })}
              className="text-aether hover:underline"
              data-testid="audit-context-editor-link"
            >
              Editor →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('gitops'), { workload: workloadFilter.trim() })}
              className="text-aether hover:underline"
              data-testid="audit-context-gitops-link"
            >
              GitOps →
            </Link>
          </>
        ) : null}
      </WorkloadContextBanner>
      <PageToolbar
        search={search}
        onSearchChange={setSearch}
        searchPlaceholder="Search action, workload, message…"
        onRefresh={() => void load()}
        refreshing={loading}
        actions={
          filteredEvents.length > 0 ? (
            <div className="flex gap-2">
              <button
                type="button"
                data-testid="audit-export-json"
                onClick={() => downloadExport('json')}
                className="inline-flex items-center gap-1.5 rounded-xl border glass-divider px-3 py-2 text-xs text-slate-300 hover:border-aether/40"
              >
                <Download className="w-3.5 h-3.5" />
                JSON
              </button>
              <button
                type="button"
                data-testid="audit-export-csv"
                onClick={() => downloadExport('csv')}
                className="inline-flex items-center gap-1.5 rounded-xl border glass-divider px-3 py-2 text-xs text-slate-300 hover:border-aether/40"
              >
                <Download className="w-3.5 h-3.5" />
                CSV
              </button>
            </div>
          ) : undefined
        }
        filters={
          <>
          <input
            type="text"
            value={workloadFilter}
            onChange={(e) => setWorkloadFilter(e.target.value)}
            placeholder="Filter by workload…"
            className="glass-select min-w-[10rem]"
          />
          {resultFilter && (
            <button
              type="button"
              data-testid="audit-clear-filters"
              onClick={() => setResultFilter('')}
              className="rounded-xl border glass-divider px-3 py-2 text-xs text-slate-400 hover:text-aether"
            >
              Clear result filter
            </button>
          )}
          </>
        }
      />

      <div className="mb-4 text-xs">
        <Link
          to={
            workloadFilter.trim()
              ? pathWithQuery(viewToPath('events'), { workload: workloadFilter.trim() })
              : viewToPath('events')
          }
          className="text-aether hover:underline"
          data-testid="audit-events-link"
        >
          View events feed →
        </Link>
        {workloadFilter.trim() ? (
          <WorkloadScopedCrossLinks workload={workloadFilter} prefix="audit" />
        ) : null}
        {' · '}
        <Link to={viewToPath('rbac')} className="text-aether hover:underline" data-testid="audit-footer-rbac-link">
          API access control →
        </Link>
      </div>

      <section className="overview-section-shell mb-6 p-6 sm:p-8">
      <div className="grid grid-cols-2 lg:grid-cols-3 xl:grid-cols-6 gap-4">
        <button type="button" onClick={() => { setResultFilter(''); setWorkloadFilter(''); setSearch(''); }} className="text-left">
          <StatCard title="Total events" value={audit.summary.total_events} color="blue" />
        </button>
        <button type="button" onClick={() => setResultFilter('success')} className="text-left">
          <StatCard title="Successes" value={audit.summary.successes} color="green" />
        </button>
        <button type="button" onClick={() => setResultFilter('failure')} className="text-left">
          <StatCard title="Failures" value={audit.summary.failures} color="red" />
        </button>
        <button type="button" onClick={() => { setResultFilter(''); setWorkloadFilter(''); }} className="text-left">
          <StatCard title="Workloads" value={audit.summary.unique_workloads} color="purple" />
        </button>
        {verify && <StatCard title="Verified" value={verify.verified} color="green" icon={<ShieldCheck size={18} />} />}
        {verify && (
          <StatCard title="Tampered" value={verify.tampered} color={verify.tampered > 0 ? 'red' : 'blue'} />
        )}
      </div>
      </section>

      {verify && (
        <div className="glass-panel-card mb-6" data-testid="audit-verify-panel">
          <div className="text-xs uppercase tracking-wider text-slate-500 mb-2">Integrity verification</div>
          <div className="flex items-center gap-3 flex-wrap">
            <Badge text={verify.integrity} variant={verify.integrity === 'VERIFIED' ? 'green' : 'red'} />
            <span className="text-sm text-slate-400">
              {verify.verified} of {verify.total} events verified successfully
            </span>
            <button
              type="button"
              data-testid="audit-reverify"
              onClick={() => void reVerify()}
              disabled={verifying}
              className="ml-auto rounded-lg border glass-divider px-3 py-1 text-xs text-slate-300 hover:border-aether/40 disabled:opacity-50"
            >
              {verifying ? 'Verifying…' : 'Re-verify integrity'}
            </button>
          </div>
          {verify.tampered_events.length > 0 && (
            <div className="mt-4 space-y-2">
              {verify.tampered_events.map((event) => (
                <div key={event.id} className="rounded-xl bg-red-500/5 border border-red-500/15 px-4 py-3 text-sm">
                  <div className="font-medium text-red-300">
                    {event.action} on {event.workload}
                  </div>
                  <div className="mt-1 text-xs text-slate-500">{formatTimestamp(event.timestamp)}</div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {audit.recent_events.length === 0 ? (
        <EmptyState icon={<Inbox size={48} />} title="No recent events" />
      ) : filteredEvents.length === 0 ? (
        <EmptyState icon={<Inbox size={48} />} title="No matching events" description="Try adjusting your search" />
      ) : (
        <div className="glass-panel-card">
          <h2 className="text-lg font-semibold text-slate-100 mb-4">Recent events</h2>
          <div className="space-y-3 max-h-[600px] overflow-auto">
            {filteredEvents.map((ev) => (
              <div key={ev.id} className="glass-panel-card flex items-start gap-3 p-4">
                <Badge text={ev.result} variant={ev.result.toLowerCase() === 'success' ? 'green' : 'red'} />
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 flex-wrap">
                    <span className="text-sm font-medium text-slate-200">{ev.action}</span>
                    <span className="text-xs text-slate-500">on</span>
                    <Link
                      to={pathWithQuery(viewToPath('workloads'), { workload: ev.workload })}
                      className="text-sm text-aether hover:underline"
                    >
                      {ev.workload}
                    </Link>
                  </div>
                  <div className="text-xs text-slate-400 mt-1">{ev.message}</div>
                  <div className="flex items-center gap-3 mt-2 text-xs text-slate-500 flex-wrap">
                    <span>{formatTimestamp(ev.timestamp)}</span>
                    {ev.runtime && <span>Runtime: {ev.runtime}</span>}
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
