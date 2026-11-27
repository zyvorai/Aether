// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { useState } from 'react';
import { Download } from 'lucide-react';
import type { OpsReport } from '../../types/operator';
import { formatMinutes, formatReadyPct, formatSeconds, queryString } from '../../utils/operator';
import { useQueryParam } from '../../utils/urlState';
import { Button } from '../ui/Button';
import { downloadText, ErrorText, Loadable, Section, useApi } from './shared';

export default function ReportTab({ refreshKey }: { refreshKey?: number }) {
  const [since, setSince] = useQueryParam('since', '7d');
  const { data, error, loading, reload } = useApi<OpsReport>(`/ops-report${queryString({ since })}`, refreshKey);
  const [failed, setFailed] = useState<string | null>(null);

  async function download(format: 'markdown' | 'html') {
    const ext = format === 'html' ? 'html' : 'md';
    const ok = await downloadText(
      `/ops-report${queryString({ since, format })}`,
      `aether-report.${ext}`,
      format === 'html' ? 'text/html' : 'text/markdown',
    );
    setFailed(ok ? null : 'The report could not be downloaded.');
  }

  const windows = ['24h', '7d', '30d'];
  return (
    <Section
      title="Operations report"
      actions={
        <div className="flex flex-wrap gap-2">
          <select aria-label="Report window" className="glass-select !w-auto" value={since} onChange={(e) => setSince(e.target.value)}>
            {(windows.includes(since) ? windows : [since, ...windows]).map((w) => (
              <option key={w} value={w}>
                Last {w}
              </option>
            ))}
          </select>
          <Button size="sm" variant="secondary" className="gap-1.5" onClick={() => void download('markdown')}>
            <Download className="h-4 w-4" /> Markdown
          </Button>
          <Button size="sm" variant="secondary" className="gap-1.5" onClick={() => void download('html')}>
            <Download className="h-4 w-4" /> HTML
          </Button>
        </div>
      }
    >
      <ErrorText>{failed}</ErrorText>
      <Loadable loading={loading} error={error} hasData={data !== null} title="Report unavailable" onRetry={() => void reload()}>
        {data ? (
          <>
            <ul className="mb-4 space-y-1 text-sm text-foreground">
              <li>
                {data.incidents.length} incident(s), mean time to recovery: {formatMinutes(data.recovery.mean_minutes)}
              </li>
              <li>{data.changes.total} change(s) recorded</li>
              <li>
                {data.audit.events_in_window} audit event(s) in the window; {data.audit.failed_integrity} of {data.audit.events_total} fail the integrity check
              </li>
            </ul>
            <div className="overflow-x-auto">
              <table className="w-full text-sm" data-testid="report-table">
                <thead className="text-left text-muted">
                  <tr>
                    {['Workload', 'Runtime', 'Ready', 'Checks', 'Incidents', 'Incident time', 'Deploys', 'Failed operations'].map((h) => (
                      <th key={h} className="px-2 py-2 text-xs font-semibold">
                        {h}
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {data.workloads.map((w) => (
                    <tr key={w.workload} className="border-t border-border">
                      <td className="px-2 py-2 font-mono text-xs">{w.workload}</td>
                      <td className="px-2 py-2">{w.runtime}</td>
                      <td className="px-2 py-2">{formatReadyPct(w.ready_pct)}</td>
                      <td className="px-2 py-2">{w.checks}</td>
                      <td className="px-2 py-2">{w.incidents}</td>
                      <td className="px-2 py-2">{formatSeconds(w.incident_secs)}</td>
                      <td className="px-2 py-2">{w.deploys}</td>
                      <td className="px-2 py-2">{w.failed_operations}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            {data.workloads.length === 0 ? <p className="mt-2 text-sm text-muted">No workloads are managed.</p> : null}
            <ul className="mt-4 list-disc space-y-1 pl-5 text-xs text-muted">
              {data.caveats.map((c) => (
                <li key={c}>{c}</li>
              ))}
            </ul>
          </>
        ) : null}
      </Loadable>
    </Section>
  );
}
