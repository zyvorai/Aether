// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import type { PlanReport } from '../../types/operator';
import { impactLabel } from '../../utils/operator';

export default function PlanTable({ plan }: { plan: PlanReport }) {
  if (plan.changes.length === 0) return <p className="text-sm text-muted">No spec field differs.</p>;
  return (
    <div className="overflow-x-auto" data-testid="plan-table">
      <table className="w-full text-sm">
        <thead className="text-left text-muted">
          <tr>
            <th className="px-2 py-2 text-xs font-semibold">Field</th>
            <th className="px-2 py-2 text-xs font-semibold">Change</th>
            {plan.summary.map((s) => (
              <th key={s.target} className="px-2 py-2 text-xs font-semibold">
                {s.target}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {plan.changes.map((c) => (
            <tr key={c.path} className="border-t border-border align-top">
              <td className="px-2 py-2 font-mono text-xs">{c.path}</td>
              <td className="break-all px-2 py-2 font-mono text-xs">
                {c.old ?? '(unset)'} → {c.new ?? '(removed)'}
              </td>
              {c.impacts.map((i) => (
                <td key={i.target} className={`px-2 py-2 text-xs ${i.impact === 'recreate' ? 'text-warning' : 'text-muted'}`} title={i.reason}>
                  {impactLabel(i.impact)}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
