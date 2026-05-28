// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useNavigate } from 'react-router';
import { viewToPath } from '../utils/dashboardRoutes';
import { pathWithQuery } from '../utils/urlState';

export function WorkloadContextBanner({
  testId,
  workload,
  description,
}: {
  testId: string;
  workload: string;
  description?: string;
}) {
  const navigate = useNavigate();
  const name = workload.trim();
  if (!name) return null;

  return (
    <div
      data-testid={testId}
      className="mb-6 rounded-xl border border-aether/30 bg-aether/5 px-4 py-3 text-sm text-slate-300"
    >
      {description ?? 'Workload context'} for <span className="font-mono text-aether">{name}</span>
      {' · '}
      <button
        type="button"
        onClick={() => navigate(pathWithQuery(viewToPath('workloads'), { workload: name }))}
        className="text-aether hover:underline"
      >
        Open workload →
      </button>
    </div>
  );
}

export function SearchQueryContextBanner({
  testId,
  query,
  entityLabel,
}: {
  testId: string;
  query: string;
  entityLabel: string;
}) {
  const navigate = useNavigate();
  const q = query.trim();
  if (!q) return null;

  return (
    <div
      data-testid={testId}
      className="mb-6 rounded-xl border border-aether/30 bg-aether/5 px-4 py-3 text-sm text-slate-300"
    >
      Filtered {entityLabel} matching <span className="font-mono text-aether">{q}</span>
      {' · '}
      <button
        type="button"
        onClick={() => navigate(pathWithQuery(viewToPath('workloads'), { workload: q }))}
        className="text-aether hover:underline"
      >
        Open workload →
      </button>
    </div>
  );
}
