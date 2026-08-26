// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

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
      className="rounded-xl border border-brand/25 bg-brand/[0.06] p-4 backdrop-blur-sm"
      data-testid={testId ?? 'zyra-action-card'}
    >
      <div className="flex items-start gap-3">
        <Sparkles className="mt-0.5 h-4 w-4 shrink-0 text-brand" />
        <div className="min-w-0 flex-1">
          <h4 className="text-sm font-semibold text-brand">{title}</h4>
          <p className="mt-1 text-sm text-ink-2">{summary}</p>
          {actions.length > 0 ? (
            <div className="mt-3 flex flex-wrap gap-2">
              {actions.map((a) => (
                <button
                  key={a.label}
                  type="button"
                  onClick={a.onClick}
                  className="rounded-lg border border-brand/30 bg-brand/10 px-3 py-1.5 text-xs font-medium text-brand hover:border-brand/50"
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
