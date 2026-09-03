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
    <section
      className="glass mb-8 border-danger/20 p-6 sm:p-8"
      data-testid="command-center-intent-sla"
    >
      <div className="mb-4 flex items-center gap-3">
        <div className="flex h-9 w-9 items-center justify-center rounded-xl border border-danger/30 bg-danger/10">
          <AlertTriangle className="h-4 w-4 text-danger" />
        </div>
        <div>
          <h3 className="text-sm font-semibold text-danger">Intent SLA breaches</h3>
          <p className="text-xs text-danger/70">{report.breaches.length} workload{report.breaches.length === 1 ? '' : 's'} out of compliance</p>
        </div>
      </div>
      <ul className="space-y-2">
        {report.breaches.slice(0, 4).map((b) => (
          <li
            key={`${b.workload}-${b.metric}`}
            className="rounded-xl border border-danger/20 bg-danger/[0.06] px-4 py-2.5 text-sm text-danger/90 backdrop-blur-sm"
          >
            <span className="font-medium">{b.workload}</span> — {b.current} vs target {b.target}
          </li>
        ))}
      </ul>
    </section>
  );
}
