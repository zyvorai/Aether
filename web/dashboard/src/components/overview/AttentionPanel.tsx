// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { useState } from 'react';
import { Link } from 'react-router';
import { AlertTriangle, ArrowRight, CheckCircle2, CircleHelp, OctagonAlert, RefreshCw } from 'lucide-react';
import { attentionVerdict, buildAttention, type AttentionSources, type AttentionVerdict } from '../../utils/attention';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery } from '../../utils/urlState';

const COLLAPSED_ROWS = 5;

// The colour comes from data-verdict on the panel (shell.css).
const VERDICT_ICON: Record<AttentionVerdict['tone'], typeof AlertTriangle> = {
  critical: OctagonAlert,
  warning: AlertTriangle,
  clear: CheckCircle2,
  unknown: CircleHelp,
};

function listWords(words: string[]): string {
  return words.length < 2 ? words.join('') : `${words.slice(0, -1).join(', ')} and ${words[words.length - 1]}`;
}

interface AttentionPanelProps {
  sources: AttentionSources;
  checkedAt: Date | null;
  refreshing: boolean;
  onRefresh: () => void;
}

/** The first thing on the Overview: what is wrong right now, and where to go. */
export default function AttentionPanel({ sources, checkedAt, refreshing, onRefresh }: AttentionPanelProps) {
  const [expanded, setExpanded] = useState(false);
  const items = buildAttention(sources);
  const verdict = attentionVerdict(items, sources);
  const Icon = VERDICT_ICON[verdict.tone];
  const shown = expanded ? items : items.slice(0, COLLAPSED_ROWS);

  return (
    <section className="glass attention" aria-labelledby="attention-headline" data-testid="overview-attention" data-verdict={verdict.tone}>
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="flex min-w-0 flex-1 basis-64 items-center gap-4">
          <span className="attention-orb" aria-hidden>
            <Icon />
          </span>
          <div className="min-w-0">
            <h2 id="attention-headline" className="attention-headline">
              {verdict.headline}
            </h2>
            <p className="mt-0.5 text-[13px] text-muted">
              {verdict.checked.length > 0 ? `Checked ${listWords(verdict.checked)}` : 'Nothing could be checked'}
              {checkedAt ? ` at ${checkedAt.toLocaleTimeString()}` : ''}.
              {verdict.unchecked.length > 0 ? (
                <span className="text-warning"> Could not load {listWords(verdict.unchecked)}.</span>
              ) : null}
            </p>
          </div>
        </div>
        <button type="button" className="btn-quiet" onClick={onRefresh} disabled={refreshing}>
          <RefreshCw className={`h-3.5 w-3.5 ${refreshing ? 'animate-spin' : ''}`} aria-hidden />
          Check again
        </button>
      </div>

      {items.length > 0 ? (
        <ul className="attention-list">
          {shown.map((item) => (
            <li key={item.id}>
              <Link to={pathWithQuery(viewToPath(item.view), item.query ?? {})} className="attention-row" data-severity={item.severity}>
                <span className="attention-dot" aria-hidden />
                <span className="min-w-0 flex-1">
                  <span className="block truncate text-sm font-medium text-foreground">
                    <span className="sr-only">{item.severity}: </span>
                    {item.title}
                  </span>
                  <span className="block truncate text-xs text-muted">{item.detail}</span>
                </span>
                <ArrowRight className="h-4 w-4 shrink-0 text-subtle" aria-hidden />
              </Link>
            </li>
          ))}
        </ul>
      ) : null}

      {items.length > COLLAPSED_ROWS ? (
        <button type="button" className="mt-2 text-[13px] text-primary hover:underline" aria-expanded={expanded} onClick={() => setExpanded((v) => !v)}>
          {expanded ? 'Show fewer' : `Show all ${items.length}`}
        </button>
      ) : null}
    </section>
  );
}
