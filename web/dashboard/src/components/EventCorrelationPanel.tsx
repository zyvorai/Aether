// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useMemo } from 'react';
import type { ClusterRelatedEvent } from '../types/api';
import { formatTimestamp } from '../utils/formatters';

interface Props {
  events: ClusterRelatedEvent[];
  resourceName: string;
}

export default function EventCorrelationPanel({ events, resourceName }: Props) {
  const summary = useMemo(() => {
    const warnings = events.filter((e) => e.type_ === 'Warning').length;
    const normal = events.length - warnings;
    const byReason = new Map<string, number>();
    for (const e of events) {
      byReason.set(e.reason, (byReason.get(e.reason) ?? 0) + 1);
    }
    const topReasons = [...byReason.entries()]
      .sort((a, b) => b[1] - a[1])
      .slice(0, 5);
    return { warnings, normal, topReasons };
  }, [events]);

  if (events.length === 0) {
    return (
      <p className="text-sm text-slate-500">No correlated events for {resourceName}.</p>
    );
  }

  return (
    <div className="space-y-4" data-testid="event-correlation-panel">
      <div className="grid grid-cols-3 gap-3 text-sm">
        <div className="glass-metric-card">
          <div className="text-xs uppercase text-slate-500">Total</div>
          <div className="font-semibold text-white">{events.length}</div>
        </div>
        <div className="glass-metric-card">
          <div className="text-xs uppercase text-slate-500">Warnings</div>
          <div className="font-semibold text-amber-400">{summary.warnings}</div>
        </div>
        <div className="glass-metric-card">
          <div className="text-xs uppercase text-slate-500">Normal</div>
          <div className="font-semibold text-emerald-400">{summary.normal}</div>
        </div>
      </div>

      {summary.topReasons.length > 0 ? (
        <div className="glass-panel-card p-4">
          <p className="mb-2 text-xs font-semibold uppercase tracking-wider text-slate-500">Top reasons</p>
          <ul className="space-y-1 text-sm">
            {summary.topReasons.map(([reason, count]) => (
              <li key={reason} className="flex justify-between gap-2 text-slate-300">
                <span>{reason}</span>
                <span className="font-mono text-slate-500">{count}</span>
              </li>
            ))}
          </ul>
        </div>
      ) : null}

      <div className="glass-table-shell max-h-64 overflow-y-auto">
        <table className="w-full border-collapse text-sm">
          <thead>
            <tr className="border-b border-slate-800 text-left text-xs uppercase tracking-wider text-slate-500">
              <th className="px-3 py-2">Time</th>
              <th className="px-3 py-2">Type</th>
              <th className="px-3 py-2">Reason</th>
              <th className="px-3 py-2">Message</th>
            </tr>
          </thead>
          <tbody>
            {events.slice(0, 50).map((e, i) => (
              <tr key={`${e.timestamp}-${i}`} className="border-b border-slate-800/50">
                <td className="whitespace-nowrap px-3 py-2 text-xs text-slate-500">{formatTimestamp(e.timestamp)}</td>
                <td className="px-3 py-2">
                  <span className={e.type_ === 'Warning' ? 'text-amber-400' : 'text-emerald-400'}>{e.type_}</span>
                </td>
                <td className="px-3 py-2 font-mono text-xs text-slate-400">{e.reason}</td>
                <td className="px-3 py-2 text-slate-300">{e.message}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
