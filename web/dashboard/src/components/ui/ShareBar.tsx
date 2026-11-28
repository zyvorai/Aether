// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import type { Tone } from '../layout/PageHero';

export interface ShareSegment {
  id: string;
  label: string;
  /** A count taken from the data; segments with nothing to show are left out. */
  count: number;
  tone: Tone;
  onSelect?: () => void;
}

interface ShareBarProps {
  title: string;
  segments: ShareSegment[];
  /** Shown instead of a bar when there is nothing to divide up. */
  empty: string;
  testId?: string;
}

/** One bar split by real counts, with a legend that carries the numbers. */
export default function ShareBar({ title, segments, empty, testId }: ShareBarProps) {
  const shown = segments.filter((s) => s.count > 0);
  const total = shown.reduce((sum, s) => sum + s.count, 0);

  return (
    <div className="min-w-0" data-testid={testId}>
      <h3 className="mb-2.5 text-[13px] font-semibold text-foreground">{title}</h3>
      {total === 0 ? (
        <p className="text-sm text-muted">{empty}</p>
      ) : (
        <>
          <div className="share-bar" aria-hidden>
            {shown.map((s) => (
              <span key={s.id} className="share-segment" data-tone={s.tone} style={{ flex: `${s.count} ${s.count} 0` }} />
            ))}
          </div>
          <ul className="share-legend -mx-2.5">
            {shown.map((s) => {
              const body = (
                <>
                  <i aria-hidden />
                  {s.label}
                  <b>{s.count}</b>
                </>
              );
              return (
                <li key={s.id}>
                  {s.onSelect ? (
                    <button type="button" className="share-key" data-tone={s.tone} onClick={s.onSelect}>
                      {body}
                    </button>
                  ) : (
                    <span className="share-key" data-tone={s.tone}>
                      {body}
                    </span>
                  )}
                </li>
              );
            })}
          </ul>
        </>
      )}
    </div>
  );
}
