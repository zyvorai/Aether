// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import type { ReactNode } from 'react';
import { cardRiseDelay } from './CardGrid';

export type EntityStatusTone = 'green' | 'red' | 'amber' | 'muted' | 'sky';

const dotTone: Record<EntityStatusTone, string> = {
  green: 'bg-emerald-400',
  red: 'bg-red-400',
  amber: 'bg-amber-400',
  muted: 'bg-slate-500',
  sky: 'bg-sky-400',
};

const accentTone: Record<EntityStatusTone, string> = {
  green: 'from-emerald-500/25 via-transparent to-transparent',
  red: 'from-red-500/25 via-transparent to-transparent',
  amber: 'from-amber-500/25 via-transparent to-transparent',
  muted: 'from-slate-500/15 via-transparent to-transparent',
  sky: 'from-sky-500/25 via-transparent to-transparent',
};

export interface EntityCardProps {
  title: string;
  subtitle?: string;
  titleTooltip?: string;
  icon?: ReactNode;
  statusTone?: EntityStatusTone;
  pulse?: boolean;
  selected?: boolean;
  index?: number;
  badge?: ReactNode;
  tags?: ReactNode;
  body?: ReactNode;
  footer?: ReactNode;
  onClick?: () => void;
  testId?: string;
}

export default function EntityCard({
  title,
  subtitle,
  titleTooltip,
  icon,
  statusTone = 'muted',
  pulse = false,
  selected = false,
  index = 0,
  badge,
  tags,
  body,
  footer,
  onClick,
  testId,
}: EntityCardProps) {
  return (
    <article
      data-testid={testId}
      style={{ animationDelay: cardRiseDelay(index) }}
      className={`card-rise group relative flex flex-col overflow-hidden rounded-2xl border transition duration-300 hover:-translate-y-0.5 ${
        selected
          ? 'border-brand/45 bg-gradient-to-br from-primary/15 via-white/[0.03] to-transparent shadow-[0_20px_50px_rgba(0,0,0,0.35)]'
          : 'glass-divider bg-gradient-to-br from-white/[0.05] via-white/[0.015] to-transparent hover:border-brand/35 hover:shadow-[0_18px_44px_rgba(0,0,0,0.32)]'
      }`}
    >
      <div className={`pointer-events-none absolute inset-0 bg-gradient-to-br ${accentTone[statusTone]}`} aria-hidden />
      <div className="pointer-events-none absolute -right-10 -top-12 h-32 w-32 rounded-full bg-brand/10 blur-3xl transition duration-500 group-hover:bg-brand/20" aria-hidden />
      <div className="pointer-events-none absolute inset-x-0 top-0 h-px bg-gradient-to-r from-transparent via-white/20 to-transparent opacity-60" aria-hidden />

      <div className="relative flex h-full flex-col p-4">
        <div className="mb-3 flex items-start gap-3">
          {icon ? (
            <div className="relative mt-0.5 flex h-10 w-10 shrink-0 items-center justify-center rounded-xl border glass-divider bg-gradient-to-br from-white/[0.08] to-transparent text-ink-2 transition group-hover:text-brand">
              {icon}
              {pulse ? (
                <span className={`absolute -right-0.5 -top-0.5 h-2.5 w-2.5 rounded-full border-2 border-[rgba(10,13,18,0.9)] platform-pulse ${dotTone[statusTone]}`} />
              ) : null}
            </div>
          ) : null}
          <div className="min-w-0 flex-1">
            <div className="flex items-start justify-between gap-2">
              <button
                type="button"
                onClick={onClick}
                className="min-w-0 flex-1 text-left"
                disabled={!onClick}
              >
                <h3 className="truncate text-base font-semibold tracking-tight text-ink transition group-hover:text-brand" title={titleTooltip ?? title}>
                  {title}
                </h3>
                {subtitle ? <p className="mt-0.5 truncate text-[11px] text-ink-3">{subtitle}</p> : null}
              </button>
              {badge ? <div className="shrink-0">{badge}</div> : null}
            </div>
            {tags ? <div className="mt-2 flex flex-wrap items-center gap-1.5">{tags}</div> : null}
          </div>
        </div>

        {body ? (
          onClick ? (
            <button type="button" onClick={onClick} className="min-w-0 flex-1 text-left">
              {body}
            </button>
          ) : (
            <div className="min-w-0 flex-1">{body}</div>
          )
        ) : null}

        {footer ? (
          <div className="mt-3 flex items-center gap-1 rounded-xl border glass-divider glass-inset-surface p-1">
            {footer}
          </div>
        ) : null}
      </div>
    </article>
  );
}
