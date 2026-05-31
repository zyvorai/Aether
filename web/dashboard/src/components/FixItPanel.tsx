// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { Wrench } from 'lucide-react';
import type { WorkloadResponse } from '../types/api';
import { inferFixRecommendations } from '../utils/k8sUx';

export type FixAction = 'logs' | 'restart' | 'scale' | 'rollback' | 'editor' | 'clusters' | 'copilot';

interface FixItPanelProps {
  workload: WorkloadResponse;
  onAction: (action: FixAction) => void;
}

export default function FixItPanel({ workload, onAction }: FixItPanelProps) {
  const fix = inferFixRecommendations(workload);
  if (!fix) return null;

  return (
    <div
      className="glass-alert-warn rounded-xl p-4"
      data-testid="fix-it-panel"
    >
      <div className="flex items-start gap-3">
        <Wrench className="text-amber-400 shrink-0 mt-0.5" size={18} />
        <div className="flex-1 min-w-0">
          <h4 className="text-sm font-semibold text-amber-100">{fix.title}</h4>
          <p className="text-sm text-amber-200/80 mt-1">{fix.summary}</p>
          <div className="mt-3 flex flex-wrap gap-2">
            {fix.actions.map((a) => (
              <button
                key={a.label}
                type="button"
                onClick={() => onAction(a.action)}
                className="btn-secondary !border-amber-500/30 !bg-amber-500/10 !text-amber-100 hover:!border-amber-400/40"
              >
                {a.label}
              </button>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
