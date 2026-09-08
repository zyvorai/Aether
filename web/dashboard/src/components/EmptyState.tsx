// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import type { ReactNode } from 'react';

interface EmptyStateProps {
  icon: ReactNode;
  title: string;
  description?: string;
  action?: ReactNode;
}

export default function EmptyState({ icon, title, description, action }: EmptyStateProps) {
  return (
    <div className="glass-empty-state flex flex-col items-center px-6 py-14 text-center sm:px-10">
      <div className="mb-3 text-subtle">{icon}</div>
      <h3 className="text-lg font-semibold text-foreground">{title}</h3>
      {description && (
        <p className="mt-2 max-w-md text-sm leading-relaxed text-subtle">{description}</p>
      )}
      {action && <div className="mt-5">{action}</div>}
    </div>
  );
}
