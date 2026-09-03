// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import type { ReactNode } from 'react';

interface EmptyStateProps {
  icon: ReactNode;
  title: string;
  description?: string;
  action?: ReactNode;
}

export default function EmptyState({ icon, title, description, action }: EmptyStateProps) {
  return (
    <div className="glass-empty-state">
      <div className="mb-3 text-ink-3">{icon}</div>
      <h3 className="text-lg font-semibold text-ink">{title}</h3>
      {description && (
        <p className="mt-2 max-w-md text-sm leading-relaxed text-ink-3">{description}</p>
      )}
      {action && <div className="mt-5">{action}</div>}
    </div>
  );
}
