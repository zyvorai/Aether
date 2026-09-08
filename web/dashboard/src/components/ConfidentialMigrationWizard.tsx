// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useEffect, useState } from 'react';
import { ArrowRight, Loader2 } from 'lucide-react';
import { apiFetch, apiPost } from '../utils/api';
import Badge from './Badge';
import type { ConfidentialFleetRow, ConfidentialMigrationPlan, ConfidentialMigrationRecord } from '../types/api';
import { useAuth } from '../contexts/AuthContext';

function toast(message: string, type: 'success' | 'error') {
  window.dispatchEvent(new CustomEvent('aether-toast', { detail: { message, type } }));
}

interface ConfidentialMigrationWizardProps {
  workloads: ConfidentialFleetRow[];
}

const TARGETS = ['kubevirt', 'kubernetes'] as const;

export default function ConfidentialMigrationWizard({ workloads }: ConfidentialMigrationWizardProps) {
  const { canMutate } = useAuth();
  const [selected, setSelected] = useState('');
  const [target, setTarget] = useState<(typeof TARGETS)[number]>('kubevirt');
  const [plan, setPlan] = useState<ConfidentialMigrationPlan | null>(null);
  const [status, setStatus] = useState<ConfidentialMigrationRecord | null>(null);
  const [loading, setLoading] = useState(false);
  const [migrating, setMigrating] = useState(false);

  const loadPlan = useCallback(async (name: string, tgt: string) => {
    if (!name) {
      setPlan(null);
      setStatus(null);
      return;
    }
    setLoading(true);
    try {
      const [p, s] = await Promise.all([
        apiFetch<ConfidentialMigrationPlan>(`/confidential/migration-plan/${encodeURIComponent(name)}/${encodeURIComponent(tgt)}`),
        apiFetch<ConfidentialMigrationRecord>(`/confidential/migration/${encodeURIComponent(name)}/status`).catch(() => null),
      ]);
      setPlan(p);
      setStatus(s);
    } catch {
      setPlan(null);
      setStatus(null);
    }
    setLoading(false);
  }, []);

  useEffect(() => {
    if (!selected && workloads.length > 0) {
      setSelected(workloads[0].workload);
    }
  }, [workloads, selected]);

  useEffect(() => {
    if (selected) {
      void loadPlan(selected, target);
    }
  }, [selected, target, loadPlan]);

  async function startMigration() {
    if (!selected || !canMutate) return;
    setMigrating(true);
    const strategy = plan?.recommended_strategy ?? 'confidential-blue-green';
    const res = await apiPost(`/workloads/${encodeURIComponent(selected)}/migrate`, {
      target_runtime: target,
      strategy,
    });
    setMigrating(false);
    if (res.success) {
      toast(`Migration of "${selected}" to ${target} started`, 'success');
      void loadPlan(selected, target);
    } else {
      toast(res.error ?? 'Migration failed', 'error');
    }
  }

  if (workloads.length === 0) {
    return (
      <p className="text-sm text-subtle">
        Deploy a confidential workload first, then plan encrypted Phase 6 migration here.
      </p>
    );
  }

  return (
    <div className="space-y-4">
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div>
          <label className="mb-1 block text-xs text-subtle">Workload</label>
          <select
            value={selected}
            onChange={(e) => setSelected(e.target.value)}
            className="glass-input"
          >
            {workloads.map((w) => (
              <option key={w.workload} value={w.workload}>
                {w.workload} ({w.runtime})
              </option>
            ))}
          </select>
        </div>
        <div>
          <label className="mb-1 block text-xs text-subtle">Target runtime</label>
          <select
            value={target}
            onChange={(e) => setTarget(e.target.value as (typeof TARGETS)[number])}
            className="glass-input"
          >
            {TARGETS.map((t) => (
              <option key={t} value={t}>
                {t}
              </option>
            ))}
          </select>
        </div>
      </div>

      {loading && (
        <p className="text-sm text-subtle flex items-center gap-2">
          <Loader2 size={14} className="animate-spin" /> Loading migration plan…
        </p>
      )}

      {plan && !loading && (
        <div className="rounded-xl glass p-4 space-y-3">
          <div className="flex flex-wrap items-center gap-2">
            <Badge
              text={plan.recommended_strategy.replace(/([A-Z])/g, '-$1').toLowerCase()}
              variant={plan.ready_for_cutover ? 'green' : 'yellow'}
            />
            {plan.ready_for_cutover ? (
              <Badge text="ready for cutover" variant="green" />
            ) : (
              <Badge text="blockers present" variant="yellow" />
            )}
          </div>
          <p className="text-xs text-muted font-mono break-all">{plan.encrypted_migration_uri}</p>
          {plan.phases.length > 0 && (
            <ol className="text-xs text-subtle space-y-1 list-decimal list-inside">
              {plan.phases.map((phase) => (
                <li key={phase}>{phase}</li>
              ))}
            </ol>
          )}
          {plan.blockers.length > 0 && (
            <ul className="text-xs text-warning/90 space-y-1">
              {plan.blockers.map((b) => (
                <li key={b}>• {b}</li>
              ))}
            </ul>
          )}
          {plan.hyper2kvm_hints.length > 0 && (
            <div className="text-xs text-subtle">
              <span className="block mb-1">External hypervisor hints:</span>
              {plan.hyper2kvm_hints.map((h) => (
                <code key={h} className="block text-muted break-all">
                  {h}
                </code>
              ))}
            </div>
          )}
          {status && (
            <p className="text-xs text-subtle">
              Last migration: {status.phase} · cutover_ready={String(status.cutover_ready)}
              {status.error ? ` · ${status.error}` : ''}
            </p>
          )}
          <button
            type="button"
            disabled={!canMutate || migrating || plan.blockers.length > 0}
            onClick={() => void startMigration()}
            className="inline-flex items-center gap-2 rounded-xl bg-success px-4 py-2 text-sm font-medium text-white hover:bg-success disabled:opacity-40"
          >
            {migrating ? 'Starting…' : (
              <>
                Start encrypted migration <ArrowRight size={14} />
              </>
            )}
          </button>
          {plan.blockers.length > 0 && (
            <p className="text-xs text-subtle">Resolve blockers before starting migration.</p>
          )}
        </div>
      )}
    </div>
  );
}
