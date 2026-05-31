// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import type { ReactNode } from 'react';

type GlassAccent = 'blue' | 'purple' | 'red' | 'neutral';
type GlassVariant = 'hero' | 'section' | 'panel';

interface GlassSectionProps {
  label?: string;
  title?: string;
  subtitle?: string;
  accent?: GlassAccent;
  variant?: GlassVariant;
  testId?: string;
  className?: string;
  actions?: ReactNode;
  icon?: ReactNode;
  children: ReactNode;
}

const shellClass: Record<GlassVariant, string> = {
  hero: 'command-center-shell',
  section: 'overview-section-shell',
  panel: 'glass-panel-card',
};

const labelClass: Record<GlassAccent, string> = {
  blue: '',
  purple: 'section-label-purple',
  red: 'section-label-red',
  neutral: 'section-label-neutral',
};

export default function GlassSection({
  label,
  title,
  subtitle,
  accent = 'blue',
  variant = 'section',
  testId,
  className = '',
  actions,
  icon,
  children,
}: GlassSectionProps) {
  const hasHeader = label || title || subtitle || actions || icon;
  const padding = variant === 'panel' ? '' : 'p-6 sm:p-8';
  const margin = variant === 'panel' ? '' : 'mb-8';

  return (
    <section
      className={`${shellClass[variant]} ${padding} ${margin} ${className}`.trim()}
      data-testid={testId}
    >
      {hasHeader ? (
        <div className={`overview-section-header flex flex-wrap items-start justify-between gap-3 ${variant === 'panel' ? 'mb-4' : 'mb-5'}`}>
          <div className="flex min-w-0 flex-1 items-start gap-3">
            {icon ? (
              <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl border border-white/10 bg-white/[0.04]">
                {icon}
              </div>
            ) : null}
            <div className="min-w-0">
              {label ? <p className={`section-label ${labelClass[accent]}`}>{label}</p> : null}
              {title ? (
                <h2 className={variant === 'hero' ? 'mt-2 text-2xl font-semibold text-white sm:text-3xl' : 'section-title'}>
                  {title}
                </h2>
              ) : null}
              {subtitle ? <p className="section-subtitle max-w-2xl">{subtitle}</p> : null}
            </div>
          </div>
          {actions ? <div className="flex shrink-0 flex-wrap items-center gap-2">{actions}</div> : null}
        </div>
      ) : null}
      {children}
    </section>
  );
}
