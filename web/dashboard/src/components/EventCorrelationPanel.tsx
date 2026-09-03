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
      <p className="text-sm text-subtle">No correlated events for {resourceName}.</p>
    );
  }

  return (
    <div className="space-y-4" data-testid="event-correlation-panel">
      <div className="grid grid-cols-3 gap-3 text-sm">
        <div className="tahoe-stat-tile rounded-[var(--radius-md)] border border-border bg-surface p-4">
          <div className="text-xs uppercase text-subtle">Total</div>
          <div className="font-semibold text-foreground">{events.length}</div>
        </div>
        <div className="tahoe-stat-tile rounded-[var(--radius-md)] border border-border bg-surface p-4">
          <div className="text-xs uppercase text-subtle">Warnings</div>
          <div className="font-semibold text-warning">{summary.warnings}</div>
        </div>
        <div className="tahoe-stat-tile rounded-[var(--radius-md)] border border-border bg-surface p-4">
          <div className="text-xs uppercase text-subtle">Normal</div>
          <div className="font-semibold text-success">{summary.normal}</div>
        </div>
      </div>

      {summary.topReasons.length > 0 ? (
        <div className="glass p-4">
          <p className="mb-2 text-xs font-semibold uppercase tracking-wider text-subtle">Top reasons</p>
          <ul className="space-y-1 text-sm">
            {summary.topReasons.map(([reason, count]) => (
              <li key={reason} className="flex justify-between gap-2 text-muted">
                <span>{reason}</span>
                <span className="font-mono text-subtle">{count}</span>
              </li>
            ))}
          </ul>
        </div>
      ) : null}

      <div className="glass-table-shell max-h-64 overflow-y-auto">
        <table className="w-full border-collapse text-sm">
          <thead>
            <tr className="glass-divider-b text-left text-xs uppercase tracking-wider text-subtle">
              <th className="px-3 py-2">Time</th>
              <th className="px-3 py-2">Type</th>
              <th className="px-3 py-2">Reason</th>
              <th className="px-3 py-2">Message</th>
            </tr>
          </thead>
          <tbody>
            {events.slice(0, 50).map((e, i) => (
              <tr key={`${e.timestamp}-${i}`} className="glass-table-row">
                <td className="whitespace-nowrap px-3 py-2 text-xs text-subtle">{formatTimestamp(e.timestamp)}</td>
                <td className="px-3 py-2">
                  <span className={e.type_ === 'Warning' ? 'text-warning' : 'text-success'}>{e.type_}</span>
                </td>
                <td className="px-3 py-2 font-mono text-xs text-muted">{e.reason}</td>
                <td className="px-3 py-2 text-muted">{e.message}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
