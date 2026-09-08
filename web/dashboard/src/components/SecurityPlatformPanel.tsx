// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useEffect, useState } from 'react';
import { Loader2, RefreshCw, Shield, ShieldAlert, Target } from 'lucide-react';
import { apiFetch, apiPost } from '../utils/api';
import GlassSection from './GlassSection';

interface SbomDriftReport {
  drift_detected: boolean;
  alerts: Array<{ field: string; severity: string }>;
  current_component_count: number;
}

interface ZeroTrustWizard {
  completion_pct: number;
  steps: Array<{ id: string; title: string; ready: boolean; workloads: string[] }>;
}

interface SecurityScoreTrend {
  current_score: number;
  trend_direction: string;
  points: Array<{ label: string; score: number }>;
}

interface ThreatHuntReport {
  configured: boolean;
  findings: Array<{ source: string; severity: string; summary: string }>;
}

export default function SecurityPlatformPanel() {
  const [tab, setTab] = useState<
    'score' | 'sbom' | 'zerotrust' | 'hunt' | 'compliance'
  >('score');
  const [loading, setLoading] = useState(true);
  const [score, setScore] = useState<SecurityScoreTrend | null>(null);
  const [sbom, setSbom] = useState<SbomDriftReport | null>(null);
  const [wizard, setWizard] = useState<ZeroTrustWizard | null>(null);
  const [huntQuery, setHuntQuery] = useState('gpu');
  const [hunt, setHunt] = useState<ThreatHuntReport | null>(null);
  const [compliance, setCompliance] = useState<{ framework: string; sections: Array<{ control: string; status: string }> } | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    const [sc, sb, zt, comp] = await Promise.all([
      apiFetch<SecurityScoreTrend>('/intelligence/security/score-trend'),
      apiFetch<SbomDriftReport>('/intelligence/security/sbom-drift'),
      apiFetch<ZeroTrustWizard>('/intelligence/security/zero-trust-wizard'),
      apiFetch<{ framework: string; sections: Array<{ control: string; status: string }> }>(
        '/intelligence/security/compliance-report',
      ),
    ]);
    setScore(sc);
    setSbom(sb);
    setWizard(zt);
    setCompliance(comp);
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function runHunt() {
    const res = await apiPost<ThreatHuntReport>('/intelligence/security/threat-hunt', {
      query: huntQuery.trim(),
      limit: 10,
    });
    if (res.success) setHunt(res.data ?? null);
  }

  async function applyPolicies(dryRun: boolean) {
    const res = await apiPost<{ applied: string[] }>('/intelligence/security/policy-apply', {
      dry_run: dryRun,
      confirm: !dryRun,
    });
    if (res.success) {
      window.dispatchEvent(
        new CustomEvent('aether-toast', {
          detail: {
            message: dryRun
              ? `Policy dry-run: ${res.data?.applied.length ?? 0} actions`
              : `Policies applied: ${res.data?.applied.length ?? 0}`,
            type: 'success',
          },
        }),
      );
    }
  }

  async function rotateSecrets(dryRun: boolean) {
    const res = await apiPost<{ rotated: string[] }>('/intelligence/security/rotation-agent', {
      dry_run: dryRun,
    });
    if (res.success) {
      window.dispatchEvent(
        new CustomEvent('aether-toast', {
          detail: {
            message: `Rotation agent: ${res.data?.rotated.length ?? 0} action(s)`,
            type: 'info',
          },
        }),
      );
    }
  }

  const tabs = [
    { id: 'score' as const, label: 'Score trend' },
    { id: 'sbom' as const, label: 'SBOM drift' },
    { id: 'zerotrust' as const, label: 'Zero-trust' },
    { id: 'hunt' as const, label: 'Threat hunt' },
    { id: 'compliance' as const, label: 'Compliance' },
  ];

  return (
    <GlassSection
      accent="purple"
      testId="security-platform-panel"
      title="Security Platform"
      subtitle="Policy apply, SBOM drift, zero-trust wizard, threat hunt, compliance"
      icon={<Shield className="h-5 w-5 text-danger" />}
      actions={
        <div className="flex flex-wrap gap-2">
          <button
            type="button"
            onClick={() => void applyPolicies(true)}
            className="rounded-xl border border-danger/30 bg-danger/10 px-3 py-2 text-xs text-danger"
            data-testid="security-policy-apply-dry-run"
          >
            Policy dry-run
          </button>
          <button
            type="button"
            onClick={() => void rotateSecrets(true)}
            className="rounded-xl border border-warning/30 bg-warning/10 px-3 py-2 text-xs text-warning"
            data-testid="security-rotation-agent"
          >
            Rotation agent
          </button>
          <button
            type="button"
            onClick={() => void load()}
            className="inline-flex items-center gap-2 rounded-xl border glass-divider px-3 py-2 text-xs text-muted hover:border-aether-ai/40"
          >
            {loading ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <RefreshCw className="h-3.5 w-3.5" />}
            Refresh
          </button>
        </div>
      }
    >
      <div className="mb-6 flex flex-wrap gap-2">
        {tabs.map((t) => (
          <button
            key={t.id}
            type="button"
            onClick={() => setTab(t.id)}
            className={`rounded-full border px-3 py-1 text-xs ${
              tab === t.id ? 'border-danger/40 bg-danger/10 text-danger' : 'glass-divider text-muted'
            }`}
          >
            {t.label}
          </button>
        ))}
      </div>

      {tab === 'score' ? (
        <div data-testid="security-score-panel" className="space-y-2 text-sm">
          <p className="text-muted">
            Fleet security score: <span className="font-semibold text-success">{score?.current_score.toFixed(1) ?? '—'}</span>
            {' · '}
            {score?.trend_direction ?? '—'}
          </p>
          <div className="flex h-20 items-end gap-1">
            {(score?.points ?? []).map((p) => (
              <div
                key={p.label}
                className={`flex-1 rounded-t ${
                  p.score >= 80 ? 'bg-success/50' : p.score >= 50 ? 'bg-warning/50' : 'bg-danger/50'
                }`}
                style={{ height: `${Math.max(8, p.score)}%` }}
                title={`${p.label}: ${p.score.toFixed(1)}`}
              />
            ))}
          </div>
        </div>
      ) : null}

      {tab === 'sbom' ? (
        <div data-testid="security-sbom-drift-panel" className="text-sm text-muted">
          {sbom?.drift_detected ? (
            <p className="text-warning">{sbom.alerts.length} SBOM drift alert(s)</p>
          ) : (
            <p className="text-success">No SBOM drift detected</p>
          )}
          <p className="text-xs text-subtle mt-1">{sbom?.current_component_count ?? 0} components tracked</p>
        </div>
      ) : null}

      {tab === 'zerotrust' ? (
        <div data-testid="security-zerotrust-panel" className="space-y-2 text-sm">
          <p className="text-muted">Rollout: {(wizard?.completion_pct ?? 0).toFixed(0)}% complete</p>
          <ul className="space-y-1 text-xs text-muted">
            {(wizard?.steps ?? []).map((s) => (
              <li key={s.id}>
                {s.ready ? '✓' : '○'} {s.title} ({s.workloads.length} workload(s))
              </li>
            ))}
          </ul>
        </div>
      ) : null}

      {tab === 'hunt' ? (
        <div data-testid="security-threat-hunt-panel">
          <div className="mb-3 flex gap-2">
            <input
              value={huntQuery}
              onChange={(e) => setHuntQuery(e.target.value)}
              className="glass-input min-w-[180px] flex-1"
              data-testid="security-hunt-input"
            />
            <button
              type="button"
              onClick={() => void runHunt()}
              className="inline-flex items-center gap-1 rounded-lg bg-danger px-3 py-2 text-xs text-white"
              data-testid="security-hunt-button"
            >
              <Target className="h-3.5 w-3.5" />
              Hunt
            </button>
          </div>
          {!hunt?.findings.length ? (
            <p className="text-sm text-subtle">
              {hunt?.configured ? 'No findings for query.' : 'Run hunt to query Aether + PacketWolf.'}
            </p>
          ) : (
            <ul className="space-y-1 text-xs text-muted">
              {hunt.findings.map((f, i) => (
                <li key={`${f.source}-${i}`} className="flex gap-2">
                  <ShieldAlert className="h-3.5 w-3.5 shrink-0 text-warning" />
                  <span>
                    [{f.source}] {f.summary}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </div>
      ) : null}

      {tab === 'compliance' ? (
        <div data-testid="security-compliance-panel" className="text-sm">
          <p className="text-muted mb-2">{compliance?.framework ?? '—'}</p>
          <ul className="space-y-1 text-xs text-muted">
            {(compliance?.sections ?? []).map((s) => (
              <li key={s.control} className="flex justify-between gap-2">
                <span>{s.control}</span>
                <span className={s.status === 'pass' ? 'text-success' : 'text-warning'}>{s.status}</span>
              </li>
            ))}
          </ul>
        </div>
      ) : null}
    </GlassSection>
  );
}
