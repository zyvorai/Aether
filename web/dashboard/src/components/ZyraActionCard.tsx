// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { Sparkles } from 'lucide-react';

interface ZyraActionCardProps {
  title: string;
  summary: string;
  actions?: { label: string; onClick: () => void }[];
  testId?: string;
}

export default function ZyraActionCard({ title, summary, actions = [], testId }: ZyraActionCardProps) {
  return (
    <div
      className="rounded-xl border border-primary/25 bg-primary/[0.06] p-4 backdrop-blur-sm"
      data-testid={testId ?? 'zyra-action-card'}
    >
      <div className="flex items-start gap-3">
        <Sparkles className="mt-0.5 h-4 w-4 shrink-0 text-primary" />
        <div className="min-w-0 flex-1">
          <h4 className="text-sm font-semibold text-primary">{title}</h4>
          <p className="mt-1 text-sm text-muted">{summary}</p>
          {actions.length > 0 ? (
            <div className="mt-3 flex flex-wrap gap-2">
              {actions.map((a) => (
                <button
                  key={a.label}
                  type="button"
                  onClick={a.onClick}
                  className="rounded-lg border border-primary/30 bg-primary/10 px-3 py-1.5 text-xs font-medium text-primary hover:border-primary/50"
                >
                  {a.label}
                </button>
              ))}
            </div>
          ) : null}
        </div>
      </div>
    </div>
  );
}
