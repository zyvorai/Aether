// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import type { ReactNode } from 'react';
import { Card, CardBody, CardHeader } from './ui/Card';
import { cn } from '../lib/cn';

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

const labelClass: Record<GlassAccent, string> = {
  blue: 'text-subtle',
  purple: 'text-subtle',
  red: 'text-danger',
  neutral: 'text-subtle',
};

/** Aurora flat section — legacy name kept for panel call sites. */
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
  const margin = variant === 'panel' ? '' : 'mb-6';

  return (
    <Card
      className={cn(margin, variant === 'hero' && 'glass-strong', className)}
      elevated={variant !== 'panel'}
      data-testid={testId}
    >
      {hasHeader ? (
        <CardHeader className="flex flex-wrap items-start justify-between gap-3 border-border">
          <div className="flex min-w-0 flex-1 items-start gap-3">
            {icon ? (
              <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl border border-border bg-surface text-primary">
                {icon}
              </div>
            ) : null}
            <div className="min-w-0">
              {label ? (
                <p className={cn('text-xs font-normal uppercase tracking-wide', labelClass[accent])}>{label}</p>
              ) : null}
              {title ? (
                <h2 className={cn('font-semibold text-foreground', variant === 'hero' ? 'mt-1 text-2xl sm:text-3xl' : 'mt-0.5 text-lg')}>
                  {title}
                </h2>
              ) : null}
              {subtitle ? <p className="mt-1 max-w-2xl text-sm text-muted">{subtitle}</p> : null}
            </div>
          </div>
          {actions ? <div className="flex shrink-0 flex-wrap items-center gap-2">{actions}</div> : null}
        </CardHeader>
      ) : null}
      <CardBody className={variant === 'panel' && !hasHeader ? 'p-0' : undefined}>{children}</CardBody>
    </Card>
  );
}
