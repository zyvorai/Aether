// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { getRuntimeBg, getSeverityColor } from '../utils/formatters';

type BadgeVariant = 'green' | 'red' | 'yellow' | 'blue' | 'purple' | 'muted' | 'accent';

interface BadgeProps {
  text: string;
  variant: BadgeVariant;
}

const variantClasses: Record<BadgeVariant, string> = {
  green: 'bg-success/10 text-success border-success/20',
  red: 'bg-danger/10 text-danger border-danger/20',
  yellow: 'bg-warning/10 text-warning border-warning/20',
  blue: 'bg-primary/10 text-primary border-primary/20',
  purple: 'bg-lavender/10 text-lavender border-lavender/20',
  muted: 'bg-hover text-muted border-border',
  accent: 'bg-primary/10 text-primary border-primary/20',
};

/** Status variants get a dot, so a state reads at a glance and long text can still wrap. */
const DOTTED: BadgeVariant[] = ['green', 'red', 'yellow'];

export default function Badge({ text, variant }: BadgeProps) {
  return (
    <span className={`pill ${variantClasses[variant]}`}>
      {DOTTED.includes(variant) ? <span className="pill-dot" aria-hidden /> : null}
      <span className="pill-text">{text}</span>
    </span>
  );
}

export function RuntimeBadge({ runtime }: { runtime: string }) {
  return (
    <span className={`pill ${getRuntimeBg(runtime)}`}>
      <span className="pill-text">{runtime}</span>
    </span>
  );
}

export function SeverityBadge({ severity }: { severity: string }) {
  return (
    <span className={`pill ${getSeverityColor(severity)}`}>
      <span className="pill-text">{severity}</span>
    </span>
  );
}
