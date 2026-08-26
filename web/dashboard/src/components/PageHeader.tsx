// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import type { ReactNode } from 'react';

export interface HeaderPill {
  label: string;
  tone?: 'brand' | 'ok' | 'warn' | 'info' | 'muted';
}

const pillTone: Record<NonNullable<HeaderPill['tone']>, string> = {
  brand: 'border-brand-wash bg-brand-wash text-brand',
  ok: 'border-emerald-500/25 bg-emerald-500/10 text-emerald-300',
  warn: 'border-amber-500/25 bg-amber-500/10 text-amber-200',
  info: 'border-sky-500/25 bg-sky-500/10 text-sky-200',
  muted: 'border-rule bg-hover text-ink-3',
};

interface PageHeaderProps {
  title: string;
  subtitle?: string;
  eyebrow?: string;
  icon?: ReactNode;
  pills?: HeaderPill[];
  actions?: ReactNode;
  children?: ReactNode;
  testId?: string;
}

export default function PageHeader({
  title,
  subtitle,
  eyebrow,
  icon,
  pills,
  actions,
  children,
  testId,
}: PageHeaderProps) {
  return (
    <section className="page-masthead mb-4 px-4 py-4 sm:px-5 sm:py-5" data-testid={testId}>
      <div className="relative z-[1] flex flex-wrap items-center justify-between gap-x-4 gap-y-3">
        <div className="flex min-w-0 items-start gap-3">
          {icon ? (
            <div className="mt-0.5 flex h-10 w-10 shrink-0 items-center justify-center rounded-xl border border-rule bg-brand-wash text-brand">
              {icon}
            </div>
          ) : null}
          <div className="min-w-0">
            {eyebrow ? (
              <div className="mb-1 inline-flex items-center gap-2 text-[11px] font-semibold uppercase tracking-[0.18em] text-brand">
                <span className="h-1.5 w-1.5 rounded-full bg-brand platform-pulse" />
                {eyebrow}
              </div>
            ) : null}
            <h2 className="truncate text-xl font-semibold tracking-[-0.02em] text-ink sm:text-2xl">{title}</h2>
            {subtitle ? (
              <p className="mt-1 max-w-2xl text-[13px] leading-relaxed text-ink-3">{subtitle}</p>
            ) : null}
          </div>
        </div>
        {pills && pills.length > 0 ? (
          <div className="flex flex-wrap items-center gap-1.5">
            {pills.map((pill) => (
              <span
                key={pill.label}
                className={`inline-flex items-center rounded-full border px-2.5 py-1 text-[11px] font-medium ${pillTone[pill.tone ?? 'muted']}`}
              >
                {pill.label}
              </span>
            ))}
          </div>
        ) : null}
        {actions ? <div className="flex flex-wrap items-center gap-2">{actions}</div> : null}
      </div>
      {children ? <div className="relative z-[1] mt-3">{children}</div> : null}
    </section>
  );
}
