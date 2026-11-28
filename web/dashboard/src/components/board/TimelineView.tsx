// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { useState } from 'react';
import { Link } from 'react-router';
import type { PlanReport } from '../../types/operator';
import type { TimelineCategory, TimelineEntry, Timeline } from '../../types/timeline';
import {
  TIMELINE_CATEGORIES,
  TIMELINE_RANGES,
  coverageLine,
  diffPath,
  entryTime,
  entryTone,
  groupByDay,
  timelinePath,
  toggleCategory,
  type TimelineRange,
} from '../../utils/timeline';
import PlanTable from '../operator/PlanTable';
import { ErrorText, Loadable, useApi } from '../operator/shared';
import SegmentedControl from '../ui/SegmentedControl';

function Revision({ workload, entry }: { workload: string; entry: TimelineEntry }) {
  const [open, setOpen] = useState(false);
  const path = diffPath(workload, entry);
  const diff = useApi<PlanReport>(open ? path : null);
  if (!path) return null;
  return (
    <div>
      <button type="button" className="timeline-link focus-ring" aria-expanded={open} onClick={() => setOpen(!open)}>
        {open ? 'Hide diff' : `Diff revision ${entry.previous_revision} → ${entry.revision}`}
      </button>
      {open ? (
        <div className="mt-2">
          <ErrorText>{diff.error}</ErrorText>
          {diff.data ? <PlanTable plan={diff.data} /> : diff.loading ? <p className="text-sm text-muted">Comparing…</p> : null}
        </div>
      ) : null}
    </div>
  );
}

function Row({ workload, entry }: { workload: string; entry: TimelineEntry }) {
  return (
    <li className="timeline-entry" data-tone={entryTone(entry)} data-kind={entry.kind}>
      <span className="timeline-dot" aria-hidden />
      <time className="timeline-time" dateTime={entry.at}>
        {entryTime(entry.at)}
      </time>
      <div className="timeline-body">
        <p className="timeline-title">
          <span className="timeline-category">{entry.category}</span>
          {entry.title}
        </p>
        {entry.detail || entry.actor ? (
          <p className="timeline-detail">
            {entry.detail}
            {entry.actor ? `${entry.detail ? ' · ' : ''}by ${entry.actor}` : ''}
          </p>
        ) : null}
        {entry.fields.length > 0 ? <p className="timeline-fields">{entry.fields.join(', ')}</p> : null}
        <Revision workload={workload} entry={entry} />
        {entry.incident !== null && entry.kind === 'incident-opened' ? (
          <Link className="timeline-link focus-ring" to={`/operations?tab=incidents&incident=${entry.incident}`}>
            Open incident {entry.incident}
          </Link>
        ) : null}
      </div>
    </li>
  );
}

export interface TimelineViewProps {
  workload: string;
  range: TimelineRange;
  only: TimelineCategory[];
  onRange: (range: TimelineRange) => void;
  onOnly: (only: TimelineCategory[]) => void;
  /** Changing this reloads without clearing what is shown. */
  refreshKey?: number;
}

export default function TimelineView({ workload, range, only, onRange, onOnly, refreshKey }: TimelineViewProps) {
  const { data, error, loading, reload } = useApi<Timeline>(timelinePath(workload, range, only), refreshKey);
  const days = data ? groupByDay(data.entries, Date.parse(data.until)) : [];
  return (
    <div className="board-timeline" data-testid="timeline">
      <div className="board-scroll">
        <SegmentedControl label="Timeline period" value={range} onChange={onRange} items={TIMELINE_RANGES.map((r) => ({ key: r, label: r }))} />
      </div>
      <div className="timeline-filters" role="group" aria-label="Show in the timeline">
        <button type="button" className="timeline-filter focus-ring" aria-pressed={only.length === 0} onClick={() => onOnly([])}>
          Everything
        </button>
        {TIMELINE_CATEGORIES.map((c) => (
          <button key={c.key} type="button" className="timeline-filter focus-ring" aria-pressed={only.includes(c.key)} onClick={() => onOnly(toggleCategory(only, c.key))}>
            {c.label}
            {data ? <span>{data.counts[c.key] ?? 0}</span> : null}
          </button>
        ))}
      </div>
      <Loadable loading={loading} error={error} hasData={data !== null} title="Timeline unavailable" onRetry={() => void reload()}>
        {data ? (
          <>
            <p className="timeline-coverage">{coverageLine(data)}</p>
            {error ? <ErrorText>Could not refresh: {error}</ErrorText> : null}
            {data.unavailable.map((u) => (
              <ErrorText key={u}>Could not read {u}</ErrorText>
            ))}
            {days.length === 0 ? (
              <p className="text-sm text-muted">Nothing recorded in this period for this filter.</p>
            ) : (
              days.map((d) => (
                <section key={d.day} aria-label={d.day}>
                  <h3 className="timeline-day">{d.day}</h3>
                  <ol className="timeline-list">
                    {d.entries.map((e, i) => (
                      <Row key={`${e.at}-${e.kind}-${e.title}-${i}`} workload={workload} entry={e} />
                    ))}
                  </ol>
                </section>
              ))
            )}
            {data.truncated ? (
              <p className="text-sm text-muted">
                Showing the newest {data.entries.length} entries. Choose a shorter period or a filter to see the rest.
              </p>
            ) : null}
          </>
        ) : null}
      </Loadable>
    </div>
  );
}
