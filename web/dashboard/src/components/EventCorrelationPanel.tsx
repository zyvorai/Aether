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
      <p className="text-sm text-zinc-500">No correlated events for {resourceName}.</p>
    );
  }

  return (
    <div className="space-y-4">
      <div className="grid grid-cols-3 gap-3 text-sm">
        <div className="rounded-lg bg-zinc-900 px-3 py-2">
          <div className="text-zinc-500 text-xs uppercase">Total</div>
          <div className="text-zinc-100 font-semibold">{events.length}</div>
        </div>
        <div className="rounded-lg bg-zinc-900 px-3 py-2">
          <div className="text-zinc-500 text-xs uppercase">Warnings</div>
          <div className="text-amber-400 font-semibold">{summary.warnings}</div>
        </div>
        <div className="rounded-lg bg-zinc-900 px-3 py-2">
          <div className="text-zinc-500 text-xs uppercase">Normal</div>
          <div className="text-emerald-400 font-semibold">{summary.normal}</div>
        </div>
      </div>

      {summary.topReasons.length > 0 && (
        <div>
          <h5 className="text-xs font-medium uppercase tracking-wider text-zinc-500 mb-2">
            Top reasons
          </h5>
          <ul className="space-y-1">
            {summary.topReasons.map(([reason, count]) => (
              <li key={reason} className="flex justify-between text-sm text-zinc-300">
                <span>{reason}</span>
                <span className="text-zinc-500">{count}×</span>
              </li>
            ))}
          </ul>
        </div>
      )}

      <div>
        <h5 className="text-xs font-medium uppercase tracking-wider text-zinc-500 mb-2">
          Timeline
        </h5>
        <div className="space-y-2 max-h-48 overflow-auto">
          {[...events]
            .sort((a, b) => (b.timestamp || '').localeCompare(a.timestamp || ''))
            .slice(0, 12)
            .map((event, index) => (
              <div
                key={`${event.timestamp}:${event.reason}:${index}`}
                className="flex gap-3 text-xs border-l-2 border-zinc-700 pl-3 py-1"
                style={{
                  borderColor: event.type_ === 'Warning' ? 'rgb(251 191 36 / 0.6)' : 'rgb(52 211 153 / 0.4)',
                }}
              >
                <div className="shrink-0 text-zinc-600 w-28">{formatTimestamp(event.timestamp)}</div>
                <div>
                  <span className={event.type_ === 'Warning' ? 'text-amber-400' : 'text-emerald-400'}>
                    {event.reason}
                  </span>
                  <p className="text-zinc-500 mt-0.5 line-clamp-2">{event.message || '—'}</p>
                </div>
              </div>
            ))}
        </div>
      </div>
    </div>
  );
}
