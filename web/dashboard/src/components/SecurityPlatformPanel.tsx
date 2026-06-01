// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from 'react';
import { Loader2, RefreshCw, Shield, ShieldAlert, Target } from 'lucide-react';
import { apiFetch, apiPost } from '../utils/api';
import GlassSection from './GlassSection';

interface SbomDriftReport {
  drift_detected: boolean;
  alerts: Array<{ field: string; severity: string }>;
  current_component_count: number;
}

interface ConfidentialFleetDashboard {
  workload_count: number;
  attestation_passed: number;
  average_trust_score: number;
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
    'score' | 'sbom' | 'confidential' | 'zerotrust' | 'hunt' | 'compliance'
  >('score');
  const [loading, setLoading] = useState(true);
  const [score, setScore] = useState<SecurityScoreTrend | null>(null);
  const [sbom, setSbom] = useState<SbomDriftReport | null>(null);
  const [confidential, setConfidential] = useState<ConfidentialFleetDashboard | null>(null);
  const [wizard, setWizard] = useState<ZeroTrustWizard | null>(null);
  const [huntQuery, setHuntQuery] = useState('gpu');
  const [hunt, setHunt] = useState<ThreatHuntReport | null>(null);
  const [compliance, setCompliance] = useState<{ framework: string; sections: Array<{ control: string; status: string }> } | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    const [sc, sb, cf, zt, comp] = await Promise.all([
      apiFetch<SecurityScoreTrend>('/intelligence/security/score-trend'),
      apiFetch<SbomDriftReport>('/intelligence/security/sbom-drift'),
      apiFetch<ConfidentialFleetDashboard>('/intelligence/security/confidential-fleet'),
      apiFetch<ZeroTrustWizard>('/intelligence/security/zero-trust-wizard'),
      apiFetch<{ framework: string; sections: Array<{ control: string; status: string }> }>(
        '/intelligence/security/compliance-report',
      ),
    ]);
    setScore(sc);
    setSbom(sb);
    setConfidential(cf);
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
    { id: 'confidential' as const, label: 'Confidential' },
    { id: 'zerotrust' as const, label: 'Zero-trust' },
    { id: 'hunt' as const, label: 'Threat hunt' },
    { id: 'compliance' as const, label: 'Compliance' },
  ];

  return (
    <GlassSection
      accent="purple"
      testId="security-platform-panel"
      title="Security Platform"
      subtitle="Policy apply, SBOM drift, confidential fleet, zero-trust wizard, threat hunt, compliance"
      icon={<Shield className="h-5 w-5 text-red-400" />}
      actions={
        <div className="flex flex-wrap gap-2">
          <button
            type="button"
            onClick={() => void applyPolicies(true)}
            className="rounded-xl border border-red-500/30 bg-red-500/10 px-3 py-2 text-xs text-red-200"
            data-testid="security-policy-apply-dry-run"
          >
            Policy dry-run
          </button>
          <button
            type="button"
            onClick={() => void rotateSecrets(true)}
            className="rounded-xl border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-xs text-amber-200"
            data-testid="security-rotation-agent"
          >
            Rotation agent
          </button>
          <button
            type="button"
            onClick={() => void load()}
            className="inline-flex items-center gap-2 rounded-xl border glass-divider px-3 py-2 text-xs text-slate-300 hover:border-aether-ai/40"
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
              tab === t.id ? 'border-red-500/40 bg-red-500/10 text-red-200' : 'glass-divider text-slate-400'
            }`}
          >
            {t.label}
          </button>
        ))}
      </div>

      {tab === 'score' ? (
        <div data-testid="security-score-panel" className="space-y-2 text-sm">
          <p className="text-slate-300">
            Fleet security score: <span className="font-semibold text-emerald-300">{score?.current_score.toFixed(1) ?? '—'}</span>
            {' · '}
            {score?.trend_direction ?? '—'}
          </p>
          <div className="flex h-20 items-end gap-1">
            {(score?.points ?? []).map((p) => (
              <div
                key={p.label}
                className="flex-1 rounded-t bg-red-500/50"
                style={{ height: `${Math.max(8, p.score)}%` }}
                title={`${p.label}: ${p.score.toFixed(1)}`}
              />
            ))}
          </div>
        </div>
      ) : null}

      {tab === 'sbom' ? (
        <div data-testid="security-sbom-drift-panel" className="text-sm text-slate-300">
          {sbom?.drift_detected ? (
            <p className="text-amber-300">{sbom.alerts.length} SBOM drift alert(s)</p>
          ) : (
            <p className="text-emerald-400">No SBOM drift detected</p>
          )}
          <p className="text-xs text-slate-500 mt-1">{sbom?.current_component_count ?? 0} components tracked</p>
        </div>
      ) : null}

      {tab === 'confidential' ? (
        <div data-testid="security-confidential-panel" className="text-sm text-slate-300">
          <p>
            {confidential?.workload_count ?? 0} confidential workload(s) ·{' '}
            {confidential?.attestation_passed ?? 0} attested · avg trust{' '}
            {(confidential?.average_trust_score ?? 0).toFixed(1)}
          </p>
        </div>
      ) : null}

      {tab === 'zerotrust' ? (
        <div data-testid="security-zerotrust-panel" className="space-y-2 text-sm">
          <p className="text-slate-300">Rollout: {(wizard?.completion_pct ?? 0).toFixed(0)}% complete</p>
          <ul className="space-y-1 text-xs text-slate-400">
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
              className="inline-flex items-center gap-1 rounded-lg bg-red-600 px-3 py-2 text-xs text-white"
              data-testid="security-hunt-button"
            >
              <Target className="h-3.5 w-3.5" />
              Hunt
            </button>
          </div>
          {!hunt?.findings.length ? (
            <p className="text-sm text-slate-500">
              {hunt?.configured ? 'No findings for query.' : 'Run hunt to query Aether + PacketWolf.'}
            </p>
          ) : (
            <ul className="space-y-1 text-xs text-slate-400">
              {hunt.findings.map((f, i) => (
                <li key={`${f.source}-${i}`} className="flex gap-2">
                  <ShieldAlert className="h-3.5 w-3.5 shrink-0 text-amber-400" />
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
          <p className="text-slate-400 mb-2">{compliance?.framework ?? '—'}</p>
          <ul className="space-y-1 text-xs text-slate-400">
            {(compliance?.sections ?? []).map((s) => (
              <li key={s.control} className="flex justify-between gap-2">
                <span>{s.control}</span>
                <span className={s.status === 'pass' ? 'text-emerald-400' : 'text-amber-300'}>{s.status}</span>
              </li>
            ))}
          </ul>
        </div>
      ) : null}
    </GlassSection>
  );
}
