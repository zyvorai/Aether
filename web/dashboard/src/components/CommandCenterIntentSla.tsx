// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useEffect, useState } from 'react';
import { AlertTriangle } from 'lucide-react';
import { apiFetch } from '../utils/api';
import type { IntentSlaBreachesReport } from '../types/api';

interface CommandCenterIntentSlaProps {
  refreshKey?: number;
}

export default function CommandCenterIntentSla({ refreshKey = 0 }: CommandCenterIntentSlaProps) {
  const [report, setReport] = useState<IntentSlaBreachesReport | null>(null);

  useEffect(() => {
    let cancelled = false;
    apiFetch<IntentSlaBreachesReport>('/intelligence/intent/sla-breaches').then((data) => {
      if (!cancelled) setReport(data);
    });
    return () => {
      cancelled = true;
    };
  }, [refreshKey]);

  if (!report?.breaches.length) return null;

  return (
    <div
      className="mb-8 rounded-[28px] border border-red-500/25 bg-red-500/10 p-4 sm:p-6"
      data-testid="command-center-intent-sla"
    >
      <div className="mb-3 flex items-center gap-2">
        <AlertTriangle className="h-4 w-4 text-red-300" />
        <h3 className="text-sm font-semibold text-red-100">Intent SLA breaches</h3>
      </div>
      <ul className="space-y-2">
        {report.breaches.slice(0, 4).map((b) => (
          <li key={`${b.workload}-${b.metric}`} className="text-sm text-red-100/90">
            <span className="font-medium">{b.workload}</span> — {b.current} vs target {b.target}
          </li>
        ))}
      </ul>
    </div>
  );
}
