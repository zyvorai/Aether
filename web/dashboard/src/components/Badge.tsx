// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

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
  muted: 'glass-inset-surface text-muted border glass-divider',
  accent: 'bg-primary/10 text-primary border-primary/20',
};

export default function Badge({ text, variant }: BadgeProps) {
  return (
    <span
      className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium border ${variantClasses[variant]}`}
    >
      {text}
    </span>
  );
}

export function RuntimeBadge({ runtime }: { runtime: string }) {
  return (
    <span
      className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium border ${getRuntimeBg(runtime)}`}
    >
      {runtime}
    </span>
  );
}

export function SeverityBadge({ severity }: { severity: string }) {
  return (
    <span
      className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium border ${getSeverityColor(severity)}`}
    >
      {severity}
    </span>
  );
}
