// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import type { ReactNode } from 'react';
import { cardRiseDelay } from './CardGrid';

export type EntityStatusTone = 'green' | 'red' | 'amber' | 'muted' | 'sky';

const dotTone: Record<EntityStatusTone, string> = {
  green: 'bg-success',
  red: 'bg-danger',
  amber: 'bg-warning',
  muted: 'bg-subtle',
  sky: 'bg-accent-fill',
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
      className={`card-rise group relative flex flex-col overflow-hidden rounded-[var(--radius-card)] border transition duration-300 hover:-translate-y-0.5 hover:shadow-[var(--shadow-raised)] ${
        selected
          ? 'border-primary bg-surface-elevated shadow-card'
          : 'border-[var(--card-edge)] bg-surface-elevated shadow-card hover:border-rule-strong'
      }`}
    >
      <div className="relative flex h-full flex-col p-4">
        <div className="mb-3 flex items-start gap-3">
          {icon ? (
            <div className="relative mt-0.5 flex h-10 w-10 shrink-0 items-center justify-center rounded-xl border glass-divider bg-surface text-muted transition group-hover:text-primary">
              {icon}
              {pulse ? (
                <span className={`absolute -right-0.5 -top-0.5 h-2.5 w-2.5 rounded-full border-2 border-surface-elevated platform-pulse ${dotTone[statusTone]}`} />
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
                <h3 className="truncate text-base font-semibold tracking-tight text-foreground transition group-hover:text-primary" title={titleTooltip ?? title}>
                  {title}
                </h3>
                {subtitle ? <p className="mt-0.5 truncate text-[11px] text-subtle">{subtitle}</p> : null}
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
