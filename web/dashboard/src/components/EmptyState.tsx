// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import type { ReactNode } from 'react';

interface EmptyStateProps {
  icon: ReactNode;
  title: string;
  description?: string;
  action?: ReactNode;
}

export default function EmptyState({ icon, title, description, action }: EmptyStateProps) {
  return (
    <div className="glass-empty-state state-panel">
      <div className="state-orb" aria-hidden>{icon}</div>
      <h3 className="state-title">{title}</h3>
      {description && <p className="state-body">{description}</p>}
      {action && <div className="mt-6">{action}</div>}
    </div>
  );
}
