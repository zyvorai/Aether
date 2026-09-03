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
  green: 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20',
  red: 'bg-red-500/10 text-red-400 border-red-500/20',
  yellow: 'bg-amber-500/10 text-amber-400 border-amber-500/20',
  blue: 'bg-blue-500/10 text-blue-400 border-blue-500/20',
  purple: 'bg-purple-500/10 text-purple-400 border-purple-500/20',
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
