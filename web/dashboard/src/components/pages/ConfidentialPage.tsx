// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from 'react';
import { ExternalLink, Lock, ShieldCheck, Terminal } from 'lucide-react';
import { Link, useNavigate } from 'react-router';
import { apiFetchSettled, apiPost } from '../../utils/api';
import { pathWithQuery, useQueryParam } from '../../utils/urlState';
import { viewToPath } from '../../utils/dashboardRoutes';
import { WorkloadContextBanner, WorkloadScopedCrossLinks } from '../QueryContextBanner';
import PageToolbar from '../PageToolbar';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import EmptyState from '../EmptyState';
import Badge from '../Badge';
import type {
  ConfidentialFleetAnalysis,
  ConfidentialFleetRow,
  ImageVerifyResult,
  KataStatus,
  MeasuredImageManifest,
  NetworkTrustScore,
  SovereignConfig,
  TeeCapabilities,
} from '../../types/api';
import { useAuth } from '../../contexts/AuthContext';
import ConfidentialMigrationWizard from '../ConfidentialMigrationWizard';

function TrustBar({ label, value }: { label: string; value: number }) {
  const pct = Math.round(value * 100);
  return (
    <div className="space-y-1">
      <div className="flex justify-between text-xs text-slate-400">
        <span>{label}</span>
        <span>{pct}%</span>
      </div>
      <div className="h-1.5 rounded-full bg-slate-800/60 overflow-hidden">
        <div
          className={`h-full rounded-full ${pct >= 80 ? 'bg-emerald-500' : pct >= 50 ? 'bg-amber-500' : 'bg-red-500'}`}
          style={{ width: `${pct}%` }}
        />
      </div>
    </div>
  );
}

function runtimeLabel(runtime: string): string {
  if (runtime === 'kubevirt') return 'KubeVirt VM';
  if (runtime === 'kubernetes') return 'Kubernetes (Kata/CoCo)';
  return runtime;
}

const CLI_IMAGE_COMMANDS = [
  'aether confidential image list',
  'aether confidential image sign my-vm ./disk.qcow2 --key cosign://aether',
  'aether confidential image verify my-vm ./disk.qcow2',
  'aether confidential image verify-digest <launch-digest>',
];

export default function ConfidentialPage() {
  const navigate = useNavigate();
  const { canMutate } = useAuth();
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [caps, setCaps] = useState<TeeCapabilities | null>(null);
  const [fleet, setFleet] = useState<ConfidentialFleetRow[]>([]);
  const [fleetTrust, setFleetTrust] = useState<NetworkTrustScore[]>([]);
  const [sovereign, setSovereign] = useState<SovereignConfig | null>(null);
  const [images, setImages] = useState<MeasuredImageManifest[]>([]);
  const [kata, setKata] = useState<KataStatus | null>(null);
  const [intel, setIntel] = useState<ConfidentialFleetAnalysis | null>(null);
  const [search, setSearch] = useQueryParam('q');
  const [workloadQuery] = useQueryParam('workload', '');
  const [verifyDigest, setVerifyDigest] = useState('');
  const [verifyName, setVerifyName] = useState('');
  const [verifyPath, setVerifyPath] = useState('');
  const [verifyResult, setVerifyResult] = useState<ImageVerifyResult | null>(null);
  const [verifyBusy, setVerifyBusy] = useState(false);
  const [signName, setSignName] = useState('');
  const [signPath, setSignPath] = useState('');
  const [signKey, setSignKey] = useState('cosign://aether');
  const [signBusy, setSignBusy] = useState(false);
  const [signMessage, setSignMessage] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setLoadFailed(false);
    const [capsRes, fleetRes, trustRes, sovereignRes, imagesRes, kataRes, intelRes] = await Promise.all([
      apiFetchSettled<TeeCapabilities>('/confidential/capabilities'),
      apiFetchSettled<ConfidentialFleetRow[]>('/confidential/fleet'),
      apiFetchSettled<NetworkTrustScore[]>('/confidential/trust-score'),
      apiFetchSettled<SovereignConfig>('/confidential/sovereign/status'),
      apiFetchSettled<MeasuredImageManifest[]>('/confidential/images'),
      apiFetchSettled<KataStatus>('/confidential/kata/status'),
      apiFetchSettled<ConfidentialFleetAnalysis>('/confidential/intelligence'),
    ]);
    if (!capsRes.ok) {
      setLoadFailed(true);
      setCaps(null);
      setFleet([]);
      setFleetTrust([]);
    } else {
      setCaps(capsRes.data);
      setFleet(fleetRes.ok ? fleetRes.data : []);
      setFleetTrust(trustRes.ok ? trustRes.data : []);
      setSovereign(sovereignRes.ok ? sovereignRes.data : null);
      setImages(imagesRes.ok ? imagesRes.data : []);
      setKata(kataRes.ok ? kataRes.data : null);
      setIntel(intelRes.ok ? intelRes.data : null);
    }
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function handleSignImage(e: React.FormEvent) {
    e.preventDefault();
    if (!canMutate) {
      setSignMessage('Read-only session — image sign is disabled');
      return;
    }
    const name = signName.trim();
    const path = signPath.trim();
    if (!name || !path) return;
    setSignBusy(true);
    setSignMessage(null);
    const res = await apiPost<MeasuredImageManifest>('/confidential/images/sign', {
      name,
      path,
      signing_key_id: signKey.trim() || 'cosign://aether',
    });
    setSignBusy(false);
    if (res.success && res.data) {
      setSignMessage(`Signed ${name} — digest ${res.data.launch_digest ?? res.data.image_hash}`);
      setImages((prev) => {
        const rest = prev.filter((img) => img.name !== name);
        return [...rest, res.data!];
      });
    } else {
      setSignMessage(res.error ?? 'Image sign failed');
    }
  }

  async function handleVerifyDigest(e: React.FormEvent) {
    e.preventDefault();
    const digest = verifyDigest.trim();
    if (!digest) return;
    setVerifyBusy(true);
    setVerifyResult(null);
    const res = await apiPost<ImageVerifyResult>('/confidential/images/verify', {
      name: 'digest-check',
      digest,
    });
    setVerifyBusy(false);
    if (res.success && res.data) {
      setVerifyResult(res.data);
    }
  }

  async function handleVerifyFile(e: React.FormEvent) {
    e.preventDefault();
    const name = verifyName.trim();
    const path = verifyPath.trim();
    if (!name || !path) return;
    setVerifyBusy(true);
    setVerifyResult(null);
    const res = await apiPost<ImageVerifyResult>('/confidential/images/verify', { name, path });
    setVerifyBusy(false);
    if (res.success && res.data) {
      setVerifyResult(res.data);
    }
  }

  const integration = caps?.integration;
  const ragnarokUiUrl = integration?.mode === 'composite' && integration.remote_url
    ? integration.remote_url.replace(/\/api\/?$/, '')
    : null;

  const filteredFleet = fleet.filter((row) => {
    const q = search.toLowerCase();
    const workloadFocus = workloadQuery.trim().toLowerCase();
    if (workloadFocus && row.workload.toLowerCase() !== workloadFocus) return false;
    if (!q) return true;
    return (
      row.workload.toLowerCase().includes(q)
      || row.runtime.toLowerCase().includes(q)
      || row.tee.toLowerCase().includes(q)
    );
  });

  const attestedFleetCount = filteredFleet.filter((row) => row.attestation_passed).length;

  if (loading && !caps && !loadFailed) {
    return <PageLoading rows={5} />;
  }

  if (loadFailed) {
    return (
      <PageLoadError
        title="Confidential APIs unavailable"
        description="Ensure aether serve is running with Ragnarok modules enabled."
        onRetry={() => void load()}
      />
    );
  }

  const host = caps?.host;

  return (
    <div>
      <PageToolbar
        search={search}
        onSearchChange={setSearch}
        searchPlaceholder="Filter confidential workloads…"
        onRefresh={() => void load()}
        refreshing={loading}
      />

      {workloadQuery.trim() ? (
        <WorkloadContextBanner
          testId="confidential-workload-context"
          workload={workloadQuery.trim()}
          description="Confidential context"
        >
          <WorkloadScopedCrossLinks
            workload={workloadQuery.trim()}
            prefix="confidential"
            showDrift
            showGitops
            showMetrics
          />
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('intelligence'), {
              workload: workloadQuery.trim(),
              tab: 'predictions',
            })}
            className="text-aether hover:underline"
            data-testid="confidential-intelligence-link"
          >
            Intelligence →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('rbac'), { workload: workloadQuery.trim() })}
            className="text-aether hover:underline"
            data-testid="confidential-rbac-link"
          >
            RBAC →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('secrets'), { workload: workloadQuery.trim() })}
            className="text-aether hover:underline"
            data-testid="confidential-secrets-link"
          >
            Secrets →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('copilot'), { workload: workloadQuery.trim(), q: `Confidential posture for ${workloadQuery.trim()}` })}
            className="text-aether hover:underline"
            data-testid="confidential-context-copilot-link"
          >
            Copilot →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('policy'), { workload: workloadQuery.trim() })}
            className="text-aether hover:underline"
            data-testid="confidential-context-policy-link"
          >
            Policy →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('audit'), { workload: workloadQuery.trim() })}
            className="text-aether hover:underline"
            data-testid="confidential-context-audit-link"
          >
            Audit →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('platform'), { workload: workloadQuery.trim() })}
            className="text-aether hover:underline"
            data-testid="confidential-context-platform-link"
          >
            Platform →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('fleet'), { workload: workloadQuery.trim() })}
            className="text-aether hover:underline"
            data-testid="confidential-context-fleet-link"
          >
            Fleet →
          </Link>
        </WorkloadContextBanner>
      ) : null}

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-4 mb-6">
        <div
          className="glass-panel-card lg:col-span-2"
          data-testid={integration?.mode === 'composite' ? 'confidential-composite-banner' : undefined}
        >
          <div className="flex items-start justify-between gap-4 mb-4">
            <div>
              <h2 className="text-lg font-semibold text-slate-100 flex items-center gap-2">
                <Lock className="w-5 h-5 text-aether" />
                Integration
              </h2>
              <p className="text-sm text-slate-500 mt-1">
                {integration?.mode === 'composite'
                  ? 'Composite deployment — Aether delegates attestation to standalone Ragnarok.'
                  : 'Embedded mode — confidential logic runs inside aether serve.'}
              </p>
            </div>
            <Badge
              text={integration?.mode === 'composite' ? 'Composite' : 'Embedded'}
              variant={integration?.mode === 'composite' ? 'yellow' : 'green'}
            />
          </div>
          {ragnarokUiUrl && (
            <a
              href={ragnarokUiUrl}
              target="_blank"
              rel="noreferrer"
              data-testid="confidential-ragnarok-link"
              className="inline-flex items-center gap-2 text-sm text-aether hover:underline"
            >
              Open Ragnarok VM console
              <ExternalLink className="w-3.5 h-3.5" />
            </a>
          )}
          {integration?.mode === 'composite' && filteredFleet.length > 0 && (
            <p
              data-testid="confidential-ragnarok-attestation-summary"
              className="mt-3 text-sm text-slate-400"
            >
              Ragnarok attestation: {attestedFleetCount}/{filteredFleet.length} workloads attested
            </p>
          )}
        </div>

        <div className="glass-panel-card">
          <h2 className="text-sm font-medium uppercase tracking-wider text-slate-400 mb-3">Host TEE</h2>
          <div className="space-y-2 text-sm">
            <div className="flex justify-between">
              <span className="text-slate-400">SEV device</span>
              <Badge text={host?.sev_device ? 'yes' : 'no'} variant={host?.sev_device ? 'green' : 'muted'} />
            </div>
            <div className="flex justify-between">
              <span className="text-slate-400">SEV-SNP</span>
              <Badge text={host?.sev_snp ? 'yes' : 'no'} variant={host?.sev_snp ? 'green' : 'muted'} />
            </div>
            <div className="flex justify-between">
              <span className="text-slate-400">Intel TDX</span>
              <Badge text={host?.tdx ? 'yes' : 'no'} variant={host?.tdx ? 'green' : 'muted'} />
            </div>
          </div>
        </div>
      </div>

      {sovereign && (sovereign.offline_attestation || sovereign.region_lock) && (
        <div className="glass-panel-card mb-6">
          <h2 className="text-lg font-semibold text-slate-100 mb-2">Sovereign mode</h2>
          <div className="flex flex-wrap gap-3 text-sm text-slate-400">
            {sovereign.offline_attestation && (
              <span className="px-2 py-1 rounded bg-slate-800/60 border border-slate-800/60">Offline attestation</span>
            )}
            {sovereign.region_lock && (
              <span className="px-2 py-1 rounded bg-slate-800/60 border border-slate-800/60">
                Region lock: {sovereign.region_lock}
              </span>
            )}
            {sovereign.byok_signing_key && (
              <span className="px-2 py-1 rounded bg-slate-800/60 border border-slate-800/60">BYOK signing active</span>
            )}
          </div>
          <p className="text-xs text-slate-600 mt-2">
            CLI: <code className="text-slate-400">aether --spec workload.yaml confidential sovereign-check</code>
          </p>
        </div>
      )}

      {kata && (
        <div className="glass-panel-card mb-6">
          <h2 className="text-lg font-semibold text-slate-100 mb-2">Kata / Confidential Containers</h2>
          <div className="flex flex-wrap gap-2 mb-3">
            <Badge text={kata.hypervisor} variant="muted" />
            <Badge
              text={kata.placement_ready ? 'TEE placement ready' : 'TEE nodes needed'}
              variant={kata.placement_ready ? 'green' : 'yellow'}
            />
          </div>
          <p className="text-xs text-slate-500 mb-2">RuntimeClasses: {kata.supported_runtime_classes.join(', ')}</p>
          <p className="text-xs text-slate-500 mb-2">Requires: {kata.operator_requirements.join(' · ')}</p>
          {kata.notes.map((n) => (
            <p key={n} className="text-xs text-slate-600">{n}</p>
          ))}
        </div>
      )}

      {fleetTrust.length > 0 && (
        <div className="glass-panel-card mb-6">
          <h2 className="text-lg font-semibold text-slate-100 mb-2">Fleet trust scores</h2>
          <p className="text-sm text-slate-500 mb-4">
            Composite trust from attestation, network policy, and firmware exposure across confidential workloads.
          </p>
          <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-3">
            {fleetTrust.map((row) => {
              const focused = workloadQuery.trim() === row.workload;
              return (
              <button
                key={row.workload}
                type="button"
                data-testid={focused ? 'confidential-workload-highlight' : undefined}
                onClick={() =>
                  navigate(
                    pathWithQuery(viewToPath('workloads'), {
                      workload: row.workload,
                      tab: 'trust',
                    }),
                  )
                }
                className={`text-left rounded-lg border bg-[#11151C] p-3 hover:border-aether/40 transition-colors ${
                  focused ? 'border-aether/50 ring-1 ring-aether/30' : 'border-slate-800/60'
                }`}
              >
                <div className="flex items-center justify-between gap-2 mb-2">
                  <span className="font-medium text-slate-100">{row.workload}</span>
                  <Badge
                    text={`${Math.round(row.composite * 100)}%`}
                    variant={row.composite >= 0.8 ? 'green' : row.composite >= 0.5 ? 'yellow' : 'red'}
                  />
                </div>
                <TrustBar label="Attestation" value={row.attestation_score} />
                <TrustBar label="Network" value={row.network_policy_score} />
              </button>
              );
            })}
          </div>
        </div>
      )}

      {intel && intel.workloads.length > 0 && (
        <div className="glass-panel-card mb-6">
          <h2 className="text-lg font-semibold text-slate-100 mb-2">AI confidential intelligence</h2>
          <p className="text-sm text-slate-500 mb-3">
            Fleet trust avg {Math.round(intel.fleet_trust_avg * 100)}% · {intel.critical_count} high-risk workload(s)
          </p>
          <div className="space-y-2 max-h-48 overflow-auto">
            {intel.workloads.map((row) => (
              <div key={row.workload} className="text-sm p-2 rounded border border-slate-800/60 bg-[#11151C]">
                <div className="flex items-center justify-between gap-2">
                  <button
                    type="button"
                    onClick={() =>
                      navigate(pathWithQuery(viewToPath('workloads'), { workload: row.workload, tab: 'trust' }))
                    }
                    className="text-slate-200 hover:text-aether text-left"
                  >
                    {row.workload}
                  </button>
                  <Badge
                    text={row.risk_level}
                    variant={row.risk_level === 'low' ? 'green' : row.risk_level === 'medium' ? 'yellow' : 'red'}
                  />
                </div>
                <p className="text-xs text-slate-500 mt-1">{row.summary}</p>
              </div>
            ))}
          </div>
        </div>
      )}

      <div className="glass-panel-card mb-6" data-testid="confidential-migration-wizard">
        <h2 className="text-lg font-semibold text-slate-100 mb-2 flex items-center gap-2">
          <Terminal className="w-5 h-5" />
          Encrypted migration (Phase 6)
        </h2>
        <p className="text-sm text-slate-500 mb-4">
          Plan and start confidential-blue-green migration from the dashboard. Example spec:{' '}
          <code className="text-slate-400">examples/confidential-migrate-kubevirt.yaml</code>
        </p>
        <ConfidentialMigrationWizard workloads={filteredFleet} />
      </div>

      <div className="grid grid-cols-1 xl:grid-cols-2 gap-6 mb-6">
        <div className="glass-panel-card" data-testid="confidential-fleet-panel">
          <h2 className="text-lg font-semibold text-slate-100 mb-1 flex items-center gap-2">
            <ShieldCheck className="w-5 h-5 text-emerald-400" />
            Confidential workloads
          </h2>
          <p className="text-sm text-slate-500 mb-4">
            Workloads with <code className="text-slate-400">confidential.enabled</code> — runtime, TEE, and image catalog status
          </p>
          <button
            type="button"
            onClick={() => navigate(viewToPath('gitops'))}
            className="mb-4 mr-4 text-xs text-aether hover:underline"
          >
            GitOps confidential sync →
          </button>
          <button
            type="button"
            data-testid="confidential-secrets-link"
            onClick={() => navigate(viewToPath('secrets'))}
            className="mb-4 mr-4 text-xs text-aether hover:underline"
          >
            Secrets vault →
          </button>
          <button
            type="button"
            data-testid="confidential-policy-link"
            onClick={() => navigate(viewToPath('policy'))}
            className="mb-4 text-xs text-aether hover:underline"
          >
            Policy check →
          </button>
          {filteredFleet.length === 0 ? (
            <EmptyState
              icon={<Lock size={40} />}
              title="No confidential workloads"
              description="Deploy a KubeVirt or Kata workload with a confidential block, or use the Visual Editor."
            />
          ) : (
            <div className="space-y-3 max-h-[28rem] overflow-auto">
              {filteredFleet.map((row) => (
                <button
                  key={row.workload}
                  type="button"
                  onClick={() =>
                    navigate(
                      pathWithQuery(viewToPath('workloads'), {
                        workload: row.workload,
                        tab: 'trust',
                      }),
                    )
                  }
                  className={`w-full text-left p-3 rounded-lg border bg-[#11151C] transition-colors ${
                    workloadQuery.trim() === row.workload
                      ? 'border-aether/60 ring-1 ring-aether/30'
                      : 'border-slate-800/60 hover:border-aether/40'
                  }`}
                  data-testid={workloadQuery.trim() === row.workload ? 'confidential-workload-highlight' : undefined}
                >
                  <div className="flex items-center justify-between mb-2 gap-2 flex-wrap">
                    <span className="font-medium text-slate-100">{row.workload}</span>
                    <div className="flex items-center gap-2 flex-wrap">
                      <Badge text={runtimeLabel(row.runtime)} variant="muted" />
                      <Badge text={row.tee} variant="muted" />
                      <span data-testid="confidential-attestation-badge">
                        <Badge
                          text={row.attestation_passed ? 'attested' : 'attestation pending'}
                          variant={row.attestation_passed ? 'green' : 'yellow'}
                        />
                      </span>
                      <Badge
                        text={`${Math.round((row.trust?.composite ?? 0) * 100)}% trust`}
                        variant={
                          (row.trust?.composite ?? 0) >= 0.8
                            ? 'green'
                            : (row.trust?.composite ?? 0) >= 0.5
                              ? 'yellow'
                              : 'red'
                        }
                      />
                    </div>
                  </div>
                  {row.image_digest ? (
                    <div className="flex items-center gap-2 mb-2 text-xs">
                      <span className="text-slate-500 shrink-0">Launch digest</span>
                      <code className="text-slate-400 truncate font-mono">{row.image_digest}</code>
                      <Badge
                        text={row.image_in_catalog ? 'in catalog' : 'not in catalog'}
                        variant={row.image_in_catalog ? 'green' : 'red'}
                      />
                    </div>
                  ) : (
                    <p className="text-xs text-slate-600 mb-2">No launch digest in spec</p>
                  )}
                  <TrustBar label="Attestation" value={row.trust.attestation_score} />
                </button>
              ))}
            </div>
          )}
        </div>

        <div className="glass-panel-card space-y-5">
          <div>
            <h2 className="text-lg font-semibold text-slate-100 mb-1">Measured images</h2>
            <p className="text-sm text-slate-500 mb-4">
              Catalog entries from <code className="text-slate-400">aether confidential image list</code>
            </p>
            {images.length === 0 ? (
              <p className="text-sm text-slate-500">No measured images registered yet.</p>
            ) : (
              <div className="overflow-auto max-h-[14rem]">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="text-left text-slate-500 border-b border-slate-800/60">
                      <th className="pb-2 pr-3">Name</th>
                      <th className="pb-2 pr-3">Launch digest</th>
                      <th className="pb-2">Signed</th>
                    </tr>
                  </thead>
                  <tbody>
                    {images.map((img) => (
                      <tr key={img.name} className="border-b border-slate-800/60">
                        <td className="py-2 pr-3 text-slate-200">{img.name}</td>
                        <td className="py-2 pr-3 font-mono text-xs text-slate-400 truncate max-w-[12rem]">
                          {img.launch_digest ?? img.image_hash}
                        </td>
                        <td className="py-2 text-slate-500 text-xs">{img.signed_at}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </div>

          <div className="border-t border-slate-800/60 pt-4">
            <h3 className="text-sm font-medium text-slate-300 mb-2">Sign measured image</h3>
            <p className="text-xs text-slate-500 mb-3">
              Registers a qcow2 on the Aether host path into the measured image catalog.
            </p>
            <form onSubmit={(e) => void handleSignImage(e)} className="space-y-2">
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
                <input
                  type="text"
                  value={signName}
                  onChange={(e) => setSignName(e.target.value)}
                  placeholder="Catalog name"
                  className="rounded bg-[#0B0E14] border border-slate-800/60 px-3 py-1.5 text-sm text-slate-200"
                />
                <input
                  type="text"
                  value={signPath}
                  onChange={(e) => setSignPath(e.target.value)}
                  placeholder="/path/on/host/disk.qcow2"
                  className="rounded bg-[#0B0E14] border border-slate-800/60 px-3 py-1.5 text-sm font-mono text-slate-200"
                />
              </div>
              <input
                type="text"
                value={signKey}
                onChange={(e) => setSignKey(e.target.value)}
                placeholder="Signing key id"
                className="w-full rounded bg-[#0B0E14] border border-slate-800/60 px-3 py-1.5 text-sm font-mono text-slate-200"
              />
              <button
                type="submit"
                disabled={signBusy || !canMutate || !signName.trim() || !signPath.trim()}
                className="btn-secondary disabled:opacity-50"
              >
                {signBusy ? 'Signing…' : 'Sign image'}
              </button>
            </form>
            {signMessage && <p className="mt-2 text-xs text-slate-400">{signMessage}</p>}
          </div>

          <div className="border-t border-slate-800/60 pt-4" data-testid="confidential-verify-form">
            <h3 className="text-sm font-medium text-slate-300 mb-2">Verify image file (host path)</h3>
            <form onSubmit={(e) => void handleVerifyFile(e)} className="space-y-2 mb-4">
              <input
                type="text"
                value={verifyName}
                onChange={(e) => setVerifyName(e.target.value)}
                placeholder="Catalog image name"
                className="w-full px-3 py-1.5 text-sm rounded bg-[#0B0E14] border border-slate-800/60 text-slate-200"
              />
              <input
                type="text"
                value={verifyPath}
                onChange={(e) => setVerifyPath(e.target.value)}
                placeholder="/path/on/server/disk.qcow2"
                className="w-full px-3 py-1.5 text-sm rounded bg-[#0B0E14] border border-slate-800/60 text-slate-200 font-mono"
              />
              <button
                type="submit"
                disabled={verifyBusy || !verifyName.trim() || !verifyPath.trim()}
                className="btn-secondary disabled:opacity-50"
              >
                {verifyBusy ? 'Verifying…' : 'Verify file'}
              </button>
            </form>
          </div>

          <div className="border-t border-slate-800/60 pt-4">
            <h3 className="text-sm font-medium text-slate-300 mb-2">Verify launch digest</h3>
            <p className="text-xs text-slate-500 mb-3">
              Dashboard equivalent of{' '}
              <code className="text-slate-400">aether confidential image verify-digest</code>
            </p>
            <form onSubmit={(e) => void handleVerifyDigest(e)} className="flex gap-2">
              <input
                type="text"
                value={verifyDigest}
                onChange={(e) => setVerifyDigest(e.target.value)}
                placeholder="sha256:… or launch digest"
                className="flex-1 min-w-0 px-3 py-1.5 text-sm rounded bg-[#0B0E14] border border-slate-800/60 text-slate-200 font-mono"
              />
              <button
                type="submit"
                disabled={verifyBusy || !verifyDigest.trim()}
                className="btn-secondary disabled:opacity-50"
              >
                {verifyBusy ? 'Checking…' : 'Verify'}
              </button>
            </form>
            {verifyResult && (
              <div
                data-testid="confidential-verify-result"
                className={`mt-3 p-2 rounded text-sm border ${verifyResult.verified ? 'border-emerald-800/60 bg-emerald-950/30 text-emerald-200' : 'border-red-800/60 bg-red-950/30 text-red-200'}`}
              >
                <Badge text={verifyResult.verified ? 'verified' : 'not found'} variant={verifyResult.verified ? 'green' : 'red'} />
                <span className="ml-2 text-xs">{verifyResult.message}</span>
              </div>
            )}
          </div>

          <div className="border-t border-slate-800/60 pt-4">
            <h3 className="text-sm font-medium text-slate-300 mb-2 flex items-center gap-2">
              <Terminal className="w-4 h-4" />
              Image CLI (host paths)
            </h3>
            <p className="text-xs text-slate-500 mb-2">
              File verify still requires host paths — use CLI or POST{' '}
              <code className="text-slate-400">/api/confidential/images/verify</code> with a path.
            </p>
            <ul className="space-y-1">
              {CLI_IMAGE_COMMANDS.map((cmd) => (
                <li key={cmd}>
                  <code className="text-xs text-slate-400 break-all">{cmd}</code>
                </li>
              ))}
            </ul>
          </div>
        </div>
      </div>

      <p className="text-xs text-slate-600">
        See docs/guides/security/RAGNAROK-AND-AETHER.md for composite bundle setup (RAGNAROK_URL).
      </p>
    </div>
  );
}
