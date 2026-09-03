// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import Badge from './Badge';
import type { PolicyResult, ValidateResponse } from '../types/api';

interface ValidateResultPanelProps {
  validate?: ValidateResponse | null;
  policy?: PolicyResult | null;
}

export default function ValidateResultPanel({ validate, policy }: ValidateResultPanelProps) {
  if (!validate && !policy) return null;

  return (
    <div className="glass mt-3 space-y-3 shrink-0 p-4" data-testid="validate-result-panel">
      {validate ? (
        <div>
          <Badge
            text={validate.valid ? 'VALID' : 'INVALID'}
            variant={validate.valid ? 'green' : 'red'}
          />
          {validate.workload_name && (
            <p className="text-sm text-muted mt-2">Workload: {validate.workload_name}</p>
          )}
          {validate.errors.length > 0 && (
            <ul className="mt-2 space-y-1">
              {validate.errors.map((e, i) => (
                <li key={i} className="text-sm text-danger">
                  - {e}
                </li>
              ))}
            </ul>
          )}
        </div>
      ) : null}

      {policy ? (
        <div>
          <Badge
            text={policy.passed ? 'POLICY OK' : 'POLICY FAILED'}
            variant={policy.passed ? 'green' : 'red'}
          />
          <p className="text-xs text-subtle mt-1">
            {policy.policies_evaluated} policies evaluated
          </p>
          {policy.violations.length > 0 && (
            <ul className="mt-2 space-y-1">
              {policy.violations.map((v, i) => (
                <li key={i} className="text-sm text-danger">
                  - [{v.policy}] {v.message}
                </li>
              ))}
            </ul>
          )}
          {policy.warnings.length > 0 && (
            <ul className="mt-2 space-y-1">
              {policy.warnings.map((w, i) => (
                <li key={i} className="text-sm text-warning">
                  - {w.message}
                </li>
              ))}
            </ul>
          )}
        </div>
      ) : null}
    </div>
  );
}
