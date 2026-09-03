import { withAuroraPage } from '../layout/AuroraPage';
// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from 'react';
import { Link, useNavigate } from 'react-router';
import { Shield, AlertTriangle, Lock, KeyRound, FileCheck } from 'lucide-react';
import { apiFetchSettled, apiPost } from '../../utils/api';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery, useQueryParam } from '../../utils/urlState';
import PageToolbar from '../PageToolbar';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import { WorkloadContextBanner, WorkloadScopedCrossLinks } from '../QueryContextBanner';
import StatCard from '../StatCard';
import Badge, { SeverityBadge } from '../Badge';
import SecurityCopilotPanel from '../SecurityCopilotPanel';
import SecurityPlatformPanel from '../SecurityPlatformPanel';
import type { SecretSummary, ThreatReport, SbomMetadata, SignedImageManifest, RemediationPlan } from '../../types/api';

function SecurityCenterPage({ refreshKey }: { refreshKey?: number } = {}) {
  const navigate = useNavigate();
  const [workloadFocus] = useQueryParam('workload');
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [hasLoadedOnce, setHasLoadedOnce] = useState(false);
  const [threats, setThreats] = useState<ThreatReport | null>(null);
  const [secrets, setSecrets] = useState<SecretSummary[]>([]);
  const [hardening, setHardening] = useState<string | null>(null);
  const [sbom, setSbom] = useState<SbomMetadata | null>(null);
  const [signedImages, setSignedImages] = useState<SignedImageManifest[]>([]);
  const [packetwolfStatus, setPacketwolfStatus] = useState<{ configured: boolean; reachable: boolean } | null>(null);
  const [remediation, setRemediation] = useState<RemediationPlan | null>(null);
  const [remediationRunning, setRemediationRunning] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    setLoadFailed(false);
    const [threatsRes, secretsRes, sbomRes, imagesRes, pwRes] = await Promise.all([
      apiFetchSettled<ThreatReport>('/intelligence/threats'),
      apiFetchSettled<SecretSummary[]>('/secrets'),
      apiFetchSettled<{ metadata: SbomMetadata }>('/security/sbom'),
      apiFetchSettled<SignedImageManifest[]>('/security/images'),
      apiFetchSettled<{ configured: boolean; reachable: boolean }>('/ecosystem/packetwolf/status'),
    ]);
    if (!threatsRes.ok && !secretsRes.ok) {
      setLoadFailed(true);
      setLoading(false);
      setHasLoadedOnce(true);
      return;
    }
    setThreats(threatsRes.ok ? threatsRes.data : null);
    setSecrets(secretsRes.ok ? secretsRes.data : []);
    setSbom(sbomRes.ok ? sbomRes.data.metadata : null);
    setSignedImages(imagesRes.ok ? imagesRes.data : []);
    setPacketwolfStatus(pwRes.ok ? pwRes.data : null);
    setLoading(false);
    setHasLoadedOnce(true);
  }, [refreshKey]);

  useEffect(() => {
    void load();
  }, [load]);

  async function generateHardeningPlan() {
    const res = await apiPost<{ reply: string }>('/copilot/chat', {
      message: 'Generate a Kubernetes hardening plan based on current threats, secrets rotation needs, and policy gaps. Be concise with actionable bullets.',
    });
    if (res.success && res.data) {
      setHardening((res.data as { reply?: string }).reply ?? JSON.stringify(res.data));
    }
  }

  async function loadRemediation() {
    const res = await apiFetchSettled<RemediationPlan>('/intelligence/remediation/plan');
    if (res.ok) setRemediation(res.data);
  }

  async function executeRemediation(dryRun: boolean) {
    setRemediationRunning(true);
    const res = await apiPost<{ executed: string[]; skipped: string[] }>('/intelligence/remediation/execute', {
      dry_run: dryRun,
      max_actions: 10,
    });
    setRemediationRunning(false);
    if (res.success && res.data) {
      const n = res.data.executed?.length ?? 0;
      window.dispatchEvent(
        new CustomEvent('aether-toast', {
          detail: { message: dryRun ? `Dry-run: ${n} actions` : `Executed ${n} remediation actions`, type: 'success' },
        }),
      );
      void loadRemediation();
    }
  }

  if (loading && !hasLoadedOnce) return <PageLoading label="Loading security center…" />;
  if (loadFailed) return <PageLoadError title="Security center unavailable" onRetry={() => void load()} />;

  const criticalThreats = threats?.threats.filter((t) => t.severity === 'critical' || t.severity === 'high') ?? [];
  const rotationNeeded = secrets.filter((s) => s.needs_rotation);
  const highRiskCount = criticalThreats.length + rotationNeeded.length;

  const hubBanner = (
    <div className="mb-6 glass-context-banner" data-testid="security-hub-context">
      Security
      {' · '}
      <Link to={viewToPath('health')} className="text-primary hover:underline" data-testid="security-context-orchestrator-link">
        Orchestrator →
      </Link>
      {' · '}
      <Link to={viewToPath('intelligence')} className="text-primary hover:underline" data-testid="security-context-intelligence-link">
        Intelligence →
      </Link>
      {' · '}
      <Link to={viewToPath('fleet')} className="text-primary hover:underline" data-testid="security-context-fleet-link">
        Fleet →
      </Link>
      {' · '}
      <Link to={viewToPath('confidential')} className="text-primary hover:underline" data-testid="security-context-confidential-hub-link">
        Confidential →
      </Link>
    </div>
  );

  return (
    <div data-testid="security-center-page">
      {hubBanner}
      {workloadFocus.trim() ? (
        <WorkloadContextBanner testId="security-workload-context" workload={workloadFocus} description="Security context">
          <WorkloadScopedCrossLinks workload={workloadFocus} prefix="security" showAudit showDrift />
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('rbac'), { workload: workloadFocus.trim() })}
            className="text-primary hover:underline"
            data-testid="security-context-rbac-link"
          >
            RBAC →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('audit'), { workload: workloadFocus.trim() })}
            className="text-primary hover:underline"
            data-testid="security-context-audit-link"
          >
            Audit →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('confidential'), { workload: workloadFocus.trim() })}
            className="text-primary hover:underline"
            data-testid="security-context-confidential-link"
          >
            Confidential →
          </Link>
        </WorkloadContextBanner>
      ) : null}
      <PageToolbar
        onRefresh={() => void load()}
        refreshing={loading}
        actions={
          <button
            type="button"
            onClick={() => void generateHardeningPlan()}
            className="btn-primary"
          >
            Generate Hardening Plan
          </button>
        }
      />

      <div className="mb-8">
        <SecurityCopilotPanel />
      </div>

      <SecurityPlatformPanel />

      <section className="glass mb-6 p-6 sm:p-8">
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
        <StatCard title="High risk items" value={highRiskCount} color="red" icon={<AlertTriangle size={18} />} />
        <StatCard title="Threats" value={threats?.threats.length ?? 0} color="yellow" icon={<Shield size={18} />} />
        <StatCard title="Secrets" value={secrets.length} color="blue" icon={<KeyRound size={18} />} />
        <StatCard title="Need rotation" value={rotationNeeded.length} color="primary" icon={<Lock size={18} />} />
      </div>
      </section>

      {hardening && (
        <div className="glass mb-6 border border-lavender/20">
          <h3 className="text-sm font-semibold text-lavender mb-2">AI Hardening Plan</h3>
          <pre className="text-sm text-muted whitespace-pre-wrap font-sans">{hardening}</pre>
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-6">
        <div className="glass" data-testid="security-sbom-card">
          <h3 className="text-lg font-semibold text-foreground mb-3 flex items-center gap-2">
            <FileCheck size={18} className="text-primary" /> SBOM
          </h3>
          {sbom ? (
            <>
              <p className="text-sm text-muted">{sbom.bom_format} {sbom.spec_version} · {sbom.component_count} components</p>
              <a href="/api/security/sbom" className="mt-3 inline-block text-sm text-primary hover:underline" download="aether-sbom.json">
                Download CycloneDX JSON
              </a>
            </>
          ) : (
            <p className="text-sm text-subtle">SBOM not generated yet. Run <code className="text-muted">aether sbom export</code>.</p>
          )}
        </div>
        <div className="glass" data-testid="security-images-card">
          <h3 className="text-lg font-semibold text-foreground mb-3">Signed images</h3>
          {signedImages.length === 0 ? (
            <p className="text-sm text-subtle">No signed VM images in catalog.</p>
          ) : (
            <ul className="space-y-2 text-sm text-muted">
              {signedImages.slice(0, 6).map((img) => (
                <li key={img.name} className="rounded border glass-divider px-3 py-2">
                  <div className="font-medium text-foreground">{img.name}</div>
                  <div className="text-xs text-subtle font-mono truncate">{img.image_hash}</div>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>

      {packetwolfStatus?.configured ? (
        <div className="glass mb-6" data-testid="security-packetwolf-card">
          <h3 className="text-lg font-semibold text-foreground mb-2">PacketWolf</h3>
          <p className="text-sm text-muted">
            Bridge {packetwolfStatus.reachable ? 'reachable' : 'unreachable'} — verify egress from Fleet expanded apps.
          </p>
        </div>
      ) : null}

      <div className="glass mb-6" data-testid="security-remediation-card">
        <div className="flex flex-wrap items-center justify-between gap-3 mb-3">
          <h3 className="text-lg font-semibold text-foreground">Anomaly remediation</h3>
          <div className="flex gap-2">
            <button type="button" onClick={() => void loadRemediation()} className="btn-secondary text-sm px-3 py-1.5">
              Plan
            </button>
            <button
              type="button"
              disabled={remediationRunning}
              onClick={() => void executeRemediation(true)}
              className="btn-secondary text-sm px-3 py-1.5"
            >
              Dry-run
            </button>
            <button
              type="button"
              disabled={remediationRunning}
              onClick={() => void executeRemediation(false)}
              className="btn-primary text-sm px-3 py-1.5"
            >
              Execute safe
            </button>
          </div>
        </div>
        {!remediation ? (
          <p className="text-sm text-muted">Load a remediation plan from PacketWolf anomalies and fleet drift.</p>
        ) : (
          <div className="space-y-2">
            <p className="text-xs text-subtle">Sources: {remediation.sources.join(', ') || 'none'}</p>
            {remediation.actions.slice(0, 6).map((a, i) => (
              <div key={`${a.action_type}-${a.target}-${i}`} className="text-sm text-muted glass-divider-t pt-2">
                <span className="font-mono text-primary">{a.action_type}</span> — {a.target}: {a.reason}
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <div className="glass">
          <h3 className="text-lg font-semibold text-foreground mb-4 flex items-center gap-2">
            <Shield size={18} className="text-primary" /> Threat scan
          </h3>
          {criticalThreats.length === 0 ? (
            <p className="text-sm text-success">No critical threats detected.</p>
          ) : (
            <div className="space-y-3">
              {criticalThreats.slice(0, 8).map((t) => (
                <div key={`${t.workload}-${t.category}`} className="rounded-lg border glass-divider p-3">
                  <div className="flex items-center justify-between gap-2 mb-1">
                    <span className="text-sm font-medium text-foreground">{t.workload}</span>
                    <SeverityBadge severity={t.severity} />
                  </div>
                  <p className="text-xs text-muted">{t.reason}</p>
                  <button
                    type="button"
                    onClick={() => navigate(pathWithQuery(viewToPath('zyra'), { workload: t.workload, q: `Fix security issue: ${t.reason}` }))}
                    className="mt-2 text-xs text-primary hover:underline"
                  >
                    Fix with Copilot →
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>

        <div className="glass">
          <h3 className="text-lg font-semibold text-foreground mb-4 flex items-center gap-2">
            <FileCheck size={18} className="text-primary" /> Policy &amp; secrets
          </h3>
          <div className="space-y-3 mb-4">
            <button
              type="button"
              onClick={() => navigate(viewToPath('policy'))}
              className="w-full text-left rounded-lg border glass-divider px-3 py-3 hover:border-primary/40"
            >
              <span className="text-sm text-foreground">Open Policy Check</span>
              <p className="text-xs text-subtle mt-0.5">Validate workloads against production rules</p>
            </button>
            <button
              type="button"
              onClick={() => navigate(viewToPath('secrets'))}
              className="w-full text-left rounded-lg border glass-divider px-3 py-3 hover:border-primary/40"
            >
              <span className="text-sm text-foreground">Manage Secrets</span>
              <p className="text-xs text-subtle mt-0.5">{rotationNeeded.length} secret(s) need rotation</p>
            </button>
            <button
              type="button"
              onClick={() => navigate(pathWithQuery(viewToPath('clusters'), { tab: 'network' }))}
              className="w-full text-left rounded-lg border glass-divider px-3 py-3 hover:border-primary/40"
            >
              <span className="text-sm text-foreground">Network policies</span>
              <p className="text-xs text-subtle mt-0.5">Review Cilium and Kubernetes network policy gaps</p>
            </button>
          </div>
          {rotationNeeded.slice(0, 5).map((s) => (
            <div key={s.name} className="flex items-center justify-between text-sm py-2 glass-divider-t">
              <span className="text-muted">{s.name}</span>
              <Badge text="rotate" variant="yellow" />
            </div>
          ))}
        </div>
      </div>

    </div>
  );
}

export default withAuroraPage('security', SecurityCenterPage);
