import { useEffect, useState } from 'react';
import { apiFetch, apiPost } from '../utils/api';
import { useAuth } from '../contexts/AuthContext';
import Badge from './Badge';
import type {
  AttestationExplain,
  AttestationStatus,
  AttestGatedSecretStatus,
  ConfidentialAnalysis,
  ConfidentialFleetRow,
  ConfidentialMigrationPlan,
  ConfidentialMigrationRecord,
  ConfidentialNetworkStatus,
  GuestKitResult,
  IsolationVerdict,
  ConfidentialPlacementAdvice,
  NetworkTrustScore,
  SecretReleaseToken,
  SovereignVerdict,
} from '../types/api';

function TrustBar({ label, value }: { label: string; value: number }) {
  const pct = Math.round(value * 100);
  return (
    <div className="space-y-1">
      <div className="flex justify-between text-xs text-zinc-400">
        <span>{label}</span>
        <span>{pct}%</span>
      </div>
      <div className="h-1.5 rounded-full bg-zinc-800 overflow-hidden">
        <div
          className={`h-full rounded-full ${pct >= 80 ? 'bg-emerald-500' : pct >= 50 ? 'bg-amber-500' : 'bg-red-500'}`}
          style={{ width: `${pct}%` }}
        />
      </div>
    </div>
  );
}

function verdictVariant(v: string): 'green' | 'red' | 'yellow' | 'muted' {
  switch (v.toLowerCase()) {
    case 'pass':
      return 'green';
    case 'fail':
      return 'red';
    case 'pending':
      return 'yellow';
    default:
      return 'muted';
  }
}

interface ConfidentialWorkloadPanelProps {
  workloadName: string;
  runtime?: string;
}

function runtimeLabel(runtime: string): string {
  if (runtime === 'kubevirt') return 'KubeVirt VM';
  if (runtime === 'kubernetes') return 'Kubernetes (Kata/CoCo)';
  return runtime;
}

export default function ConfidentialWorkloadPanel({ workloadName, runtime }: ConfidentialWorkloadPanelProps) {
  const { canMutate } = useAuth();
  const [loading, setLoading] = useState(true);
  const [meta, setMeta] = useState<ConfidentialFleetRow | null>(null);
  const [trust, setTrust] = useState<NetworkTrustScore | null>(null);
  const [status, setStatus] = useState<AttestationStatus | null>(null);
  const [explain, setExplain] = useState<AttestationExplain | null>(null);
  const [secretStatus, setSecretStatus] = useState<AttestGatedSecretStatus[]>([]);
  const [isolation, setIsolation] = useState<IsolationVerdict | null>(null);
  const [placement, setPlacement] = useState<ConfidentialPlacementAdvice | null>(null);
  const [showExplain, setShowExplain] = useState(false);
  const [notConfidential, setNotConfidential] = useState(false);
  const [guestkitHistory, setGuestkitHistory] = useState<GuestKitResult[]>([]);
  const [network, setNetwork] = useState<ConfidentialNetworkStatus | null>(null);
  const [analysis, setAnalysis] = useState<ConfidentialAnalysis | null>(null);
  const [migrationPlan, setMigrationPlan] = useState<ConfidentialMigrationPlan | null>(null);
  const [migrationStatus, setMigrationStatus] = useState<ConfidentialMigrationRecord | null>(null);
  const [guestkitBusy, setGuestkitBusy] = useState(false);
  const [guestkitMessage, setGuestkitMessage] = useState<string | null>(null);
  const [sovereignVerdict, setSovereignVerdict] = useState<SovereignVerdict | null>(null);
  const [sovereignBusy, setSovereignBusy] = useState(false);
  const [secretActionMsg, setSecretActionMsg] = useState<string | null>(null);
  const [secretReleaseBusy, setSecretReleaseBusy] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    async function load() {
      setLoading(true);
      setNotConfidential(false);
      const [metaData, trustData, statusData, secretsData, isolationData, placementData, gkHistory, netData, intelData, migPlan, migStatus] = await Promise.all([
        apiFetch<ConfidentialFleetRow>(`/confidential/workload/${encodeURIComponent(workloadName)}`),
        apiFetch<NetworkTrustScore>(`/confidential/trust-score/${encodeURIComponent(workloadName)}`),
        apiFetch<AttestationStatus>(`/confidential/attestation/${encodeURIComponent(workloadName)}/status`),
        apiFetch<AttestGatedSecretStatus[]>(`/confidential/secrets/${encodeURIComponent(workloadName)}/status`),
        apiFetch<IsolationVerdict>(`/confidential/isolation/${encodeURIComponent(workloadName)}`),
        apiFetch<ConfidentialPlacementAdvice>(`/confidential/placement/${encodeURIComponent(workloadName)}`),
        apiFetch<GuestKitResult[]>(`/confidential/guestkit/${encodeURIComponent(workloadName)}/history`),
        apiFetch<ConfidentialNetworkStatus>(`/confidential/network/${encodeURIComponent(workloadName)}`),
        apiFetch<ConfidentialAnalysis>(`/confidential/intelligence/${encodeURIComponent(workloadName)}`),
        apiFetch<ConfidentialMigrationPlan>(`/confidential/migration-plan/${encodeURIComponent(workloadName)}/kubevirt`),
        apiFetch<ConfidentialMigrationRecord>(`/confidential/migration/${encodeURIComponent(workloadName)}/status`),
      ]);
      if (cancelled) return;
      if (!metaData && !trustData) {
        setNotConfidential(true);
      }
      setMeta(metaData);
      setTrust(metaData?.trust ?? trustData);
      setStatus(statusData);
      setSecretStatus(secretsData ?? []);
      setIsolation(isolationData);
      setPlacement(placementData);
      setGuestkitHistory(gkHistory ?? []);
      setNetwork(netData);
      setAnalysis(intelData);
      setMigrationPlan(migPlan);
      setMigrationStatus(migStatus);
      setExplain(null);
      setShowExplain(false);
      setLoading(false);
    }
    void load();
    return () => {
      cancelled = true;
    };
  }, [workloadName]);

  async function runSovereignCheck() {
    setSovereignBusy(true);
    const data = await apiFetch<SovereignVerdict>(
      `/confidential/sovereign/evaluate/${encodeURIComponent(workloadName)}`,
    );
    setSovereignVerdict(data);
    setSovereignBusy(false);
  }

  async function releaseSecret(secretName: string, provider: string) {
    if (!canMutate) {
      setSecretActionMsg('Read-only session — secret release is disabled');
      return;
    }
    setSecretReleaseBusy(secretName);
    setSecretActionMsg(null);
    const res = await apiPost<SecretReleaseToken>('/confidential/secrets/release', {
      vm_id: workloadName,
      secret_name: secretName,
      provider: provider || 'vault',
    });
    setSecretReleaseBusy(null);
    if (res.success && res.data) {
      setSecretActionMsg(`Released ${secretName} (expires ${res.data.expires_at})`);
      const refreshed = await apiFetch<AttestGatedSecretStatus[]>(
        `/confidential/secrets/${encodeURIComponent(workloadName)}/status`,
      );
      if (refreshed) setSecretStatus(refreshed);
    } else {
      setSecretActionMsg(res.error ?? 'Secret release failed');
    }
  }

  async function runGuestkit(mode: string) {
    setGuestkitBusy(true);
    setGuestkitMessage(null);
    const res = await apiPost<GuestKitResult>('/confidential/guestkit/inspect', {
      vm_id: workloadName,
      mode,
    });
    setGuestkitBusy(false);
    if (res.success && res.data) {
      setGuestkitHistory((prev) => [...prev, res.data!]);
      setGuestkitMessage(
        res.data.passed
          ? `GuestKit ${mode} passed`
          : `GuestKit ${mode} failed: ${res.data.findings.join('; ')}`,
      );
    } else {
      setGuestkitMessage(res.error ?? 'GuestKit inspection failed');
    }
  }

  async function loadExplain() {
    if (explain) {
      setShowExplain((v) => !v);
      return;
    }
    const data = await apiFetch<AttestationExplain>(
      `/confidential/attestation/${encodeURIComponent(workloadName)}/explain`,
    );
    if (data) {
      setExplain(data);
      setShowExplain(true);
    }
  }

  if (loading) {
    return <p className="text-sm text-zinc-500 py-4">Loading trust and attestation data…</p>;
  }

  if (notConfidential && !status) {
    return (
      <div className="py-4 space-y-2">
        <p className="text-sm text-zinc-400">
          This workload is not configured for confidential computing. Add a{' '}
          <code className="text-zinc-300">confidential:</code> block with{' '}
          <code className="text-zinc-300">enabled: true</code> in the workload spec (KubeVirt runtime).
        </p>
        <p className="text-xs text-zinc-600">
          Use the Visual Editor with runtime KubeVirt, or see <code>examples/confidential-snp.yaml</code>.
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-6 py-2">
      {(meta || runtime) && (
        <div className="flex flex-wrap items-center gap-2 pb-2 border-b border-zinc-800">
          <Badge text={runtimeLabel(meta?.runtime ?? runtime ?? 'unknown')} variant="muted" />
          {meta?.tee && <Badge text={meta.tee} variant="muted" />}
          {meta?.attestation_passed !== undefined && (
            <Badge
              text={meta.attestation_passed ? 'attestation pass' : 'attestation pending'}
              variant={meta.attestation_passed ? 'green' : 'yellow'}
            />
          )}
        </div>
      )}

      {analysis && (
        <div className="border-b border-zinc-800 pb-4">
          <h4 className="text-sm font-medium text-zinc-300 mb-2">AI confidential analysis</h4>
          <Badge
            text={`${analysis.risk_level} risk · ${Math.round(analysis.trust_composite * 100)}% trust`}
            variant={analysis.risk_level === 'low' ? 'green' : analysis.risk_level === 'medium' ? 'yellow' : 'red'}
          />
          <p className="text-xs text-zinc-500 mt-2">{analysis.summary}</p>
          {analysis.recommendations.length > 0 && (
            <ul className="mt-2 space-y-1 text-xs text-amber-200/90">
              {analysis.recommendations.slice(0, 3).map((r) => (
                <li key={r}>{r}</li>
              ))}
            </ul>
          )}
        </div>
      )}

      {placement && placement.confidential_enabled && (
        <div className="border-b border-zinc-800 pb-4">
          <h4 className="text-sm font-medium text-zinc-300 mb-2">Trust-aware placement</h4>
          <div className="flex flex-wrap gap-2 mb-2">
            <Badge text={`runtime: ${placement.recommended_runtime}`} variant="muted" />
            <Badge
              text={placement.host_tee_ready ? 'host TEE ready' : 'host TEE missing'}
              variant={placement.host_tee_ready ? 'green' : 'red'}
            />
            {placement.kata_runtime_class && (
              <Badge text={placement.kata_runtime_class} variant="muted" />
            )}
          </div>
          {placement.blockers.length > 0 && (
            <ul className="text-xs text-red-300/90 space-y-1 mb-2">
              {placement.blockers.map((b) => (
                <li key={b}>{b}</li>
              ))}
            </ul>
          )}
          {placement.gitops_issues.length > 0 && (
            <ul className="text-xs text-amber-300/90 space-y-1 mb-2">
              {placement.gitops_issues.map((issue) => (
                <li key={issue}>GitOps: {issue}</li>
              ))}
            </ul>
          )}
          {placement.schedule_constraints.length > 0 && (
            <p className="text-xs text-zinc-500 font-mono">
              {placement.schedule_constraints.join(' · ')}
            </p>
          )}
        </div>
      )}

      <div className="border-b border-zinc-800 pb-4">
        <div className="flex flex-wrap items-center justify-between gap-2 mb-2">
          <h4 className="text-sm font-medium text-zinc-300">Sovereign compliance</h4>
          <button
            type="button"
            disabled={sovereignBusy}
            onClick={() => void runSovereignCheck()}
            className="rounded border border-zinc-600 px-2.5 py-1 text-xs text-zinc-300 hover:border-aether/50 disabled:opacity-50"
          >
            {sovereignBusy ? 'Evaluating…' : 'Run sovereign check'}
          </button>
        </div>
        {sovereignVerdict ? (
          <div className="space-y-2 text-sm">
            <Badge
              text={sovereignVerdict.compliant ? 'compliant' : 'violations'}
              variant={sovereignVerdict.compliant ? 'green' : 'red'}
            />
            {sovereignVerdict.violations.length > 0 && (
              <ul className="space-y-1 text-xs text-red-300/90">
                {sovereignVerdict.violations.map((v) => (
                  <li key={v}>{v}</li>
                ))}
              </ul>
            )}
            {sovereignVerdict.hints.length > 0 && (
              <ul className="space-y-1 text-xs text-zinc-500">
                {sovereignVerdict.hints.map((h) => (
                  <li key={h}>{h}</li>
                ))}
              </ul>
            )}
          </div>
        ) : (
          <p className="text-xs text-zinc-500">Evaluate region lock, BYOK, and offline attestation policy for this spec.</p>
        )}
      </div>

      {migrationPlan && (
        <div className="border-b border-zinc-800 pb-4">
          <h4 className="text-sm font-medium text-zinc-300 mb-2">Encrypted migration plan</h4>
          <Badge
            text={migrationPlan.recommended_strategy.replace(/([A-Z])/g, '-$1').toLowerCase()}
            variant={migrationPlan.ready_for_cutover ? 'green' : 'yellow'}
          />
          <p className="text-xs font-mono text-zinc-500 break-all mt-2">
            {migrationPlan.encrypted_migration_uri}
          </p>
          {migrationPlan.blockers.length > 0 && (
            <ul className="mt-2 space-y-1 text-xs text-red-300/90">
              {migrationPlan.blockers.map((b) => (
                <li key={b}>{b}</li>
              ))}
            </ul>
          )}
          {migrationStatus && (
            <p className="text-xs text-zinc-500 mt-2">
              Last migration: {migrationStatus.phase} · cutover_ready={String(migrationStatus.cutover_ready)}
            </p>
          )}
          <p className="mt-2 text-xs text-zinc-600 font-mono">
            aether --spec workload.yaml confidential migration plan
          </p>
        </div>
      )}

      {network && (
        <div className="border-b border-zinc-800 pb-4">
          <h4 className="text-sm font-medium text-zinc-300 mb-2">Zero-trust network</h4>
          <div className="flex flex-wrap gap-2 mb-2">
            <Badge text={`${network.policy_count} policies`} variant="muted" />
            {network.cilium_auto_policy && <Badge text="auto Cilium" variant="green" />}
          </div>
          {network.spiffe_id && (
            <p className="text-xs font-mono text-zinc-500 break-all mb-2">{network.spiffe_id}</p>
          )}
          {network.recommendations.map((r) => (
            <p key={r} className="text-xs text-zinc-600">{r}</p>
          ))}
        </div>
      )}

      {meta?.image_digest && (
        <div>
          <h4 className="text-sm font-medium text-zinc-300 mb-2">Measured launch digest</h4>
          <code className="text-xs text-zinc-400 break-all block mb-2">{meta.image_digest}</code>
          <Badge
            text={meta.image_in_catalog ? 'in verified catalog' : 'not in catalog — deploy blocked'}
            variant={meta.image_in_catalog ? 'green' : 'red'}
          />
        </div>
      )}

      {trust && (
        <div>
          <div className="flex items-center justify-between mb-3">
            <h4 className="text-sm font-medium text-zinc-300">Composite trust score</h4>
            <Badge
              text={`${Math.round(trust.composite * 100)}%`}
              variant={trust.composite >= 0.8 ? 'green' : trust.composite >= 0.5 ? 'yellow' : 'red'}
            />
          </div>
          <div className="space-y-3">
            <TrustBar label="Attestation" value={trust.attestation_score} />
            <TrustBar label="Network policy" value={trust.network_policy_score} />
            <TrustBar label="Firmware exposure" value={trust.firmware_exposure_score} />
          </div>
          {trust.spiffe_id && (
            <p className="mt-3 text-xs font-mono text-zinc-500 break-all">SPIFFE: {trust.spiffe_id}</p>
          )}
        </div>
      )}

      {status ? (
        <div className="border-t border-zinc-700 pt-4">
          <h4 className="text-sm font-medium text-zinc-300 mb-3">Attestation status</h4>
          <div className="grid grid-cols-2 gap-3 text-sm">
            <div>
              <span className="text-zinc-500 block text-xs mb-1">Verdict</span>
              <Badge text={status.last_verdict} variant={verdictVariant(status.last_verdict)} />
            </div>
            <div>
              <span className="text-zinc-500 block text-xs mb-1">Measurement drift</span>
              <Badge text={status.drift ? 'drift detected' : 'stable'} variant={status.drift ? 'red' : 'green'} />
            </div>
            {status.baseline_digest && (
              <div className="col-span-2">
                <span className="text-zinc-500 block text-xs mb-1">Baseline digest</span>
                <code className="text-xs text-zinc-400 break-all">{status.baseline_digest}</code>
              </div>
            )}
            <div className="col-span-2 text-xs text-zinc-600">Updated {status.updated_at}</div>
          </div>
          <button
            type="button"
            onClick={() => void loadExplain()}
            className="mt-3 text-xs text-aether hover:underline"
          >
            {showExplain ? 'Hide attestation explain' : 'Explain attestation'}
          </button>
          {showExplain && explain && (
            <div className="mt-3 p-3 rounded-lg bg-zinc-950 border border-zinc-700 text-sm">
              <p className="text-zinc-300 mb-2">{explain.summary}</p>
              {explain.failure_reasons.length > 0 && (
                <ul className="space-y-1 text-xs text-red-300/90">
                  {explain.failure_reasons.map((r) => (
                    <li key={r.code}>
                      [{r.severity}] {r.message}
                    </li>
                  ))}
                </ul>
              )}
              {explain.guestkit && (
                <div className="mt-3 pt-3 border-t border-zinc-800">
                  <p className="text-xs text-zinc-500 mb-1">GuestKit ({explain.guestkit.last_mode})</p>
                  <Badge text={explain.guestkit.passed ? 'passed' : 'failed'} variant={explain.guestkit.passed ? 'green' : 'red'} />
                  {explain.guestkit.repair_steps.length > 0 && (
                    <ul className="mt-2 space-y-1 text-xs text-amber-200/90">
                      {explain.guestkit.repair_steps.map((step) => (
                        <li key={step}>{step}</li>
                      ))}
                    </ul>
                  )}
                </div>
              )}
            </div>
          )}
        </div>
      ) : (
        <p className="text-sm text-zinc-500 border-t border-zinc-700 pt-4">
          No attestation record yet. Submit a guest report via Ragnarok or POST{' '}
          <code className="text-zinc-400">/api/confidential/attestation/verify</code>.
        </p>
      )}

      <div className="border-t border-zinc-700 pt-4">
        <h4 className="text-sm font-medium text-zinc-300 mb-2">GuestKit offline inspection</h4>
        <p className="text-xs text-zinc-500 mb-3">
          Pre-launch digest check (uses spec launch digest). File paths require CLI on the Aether host.
        </p>
        <div className="flex flex-wrap gap-2 mb-3">
          <button
            type="button"
            disabled={guestkitBusy}
            onClick={() => void runGuestkit('pre-launch')}
            className="px-2.5 py-1 text-xs rounded border border-zinc-600 text-zinc-300 hover:border-aether/50 disabled:opacity-50"
          >
            Pre-launch check
          </button>
          <button
            type="button"
            disabled={guestkitBusy}
            onClick={() => void runGuestkit('attested-repair')}
            className="px-2.5 py-1 text-xs rounded border border-zinc-600 text-zinc-300 hover:border-aether/50 disabled:opacity-50"
          >
            Repair playbook
          </button>
        </div>
        {guestkitMessage && (
          <p className="text-xs text-zinc-400 mb-2">{guestkitMessage}</p>
        )}
        {guestkitHistory.length > 0 && (
          <div className="space-y-2 max-h-40 overflow-auto">
            {guestkitHistory.slice().reverse().map((entry) => (
              <div key={entry.inspected_at} className="text-xs p-2 rounded bg-zinc-950 border border-zinc-800">
                <div className="flex items-center justify-between gap-2 mb-1">
                  <span className="text-zinc-400">{entry.mode}</span>
                  <Badge text={entry.passed ? 'pass' : 'fail'} variant={entry.passed ? 'green' : 'red'} />
                </div>
                <p className="text-zinc-500">{entry.findings.join(' · ')}</p>
              </div>
            ))}
          </div>
        )}
        <p className="mt-2 text-xs text-zinc-600 font-mono">
          aether confidential guestkit inspect {workloadName} --mode pre-launch --image ./disk.qcow2
        </p>
      </div>

      {isolation && (
        <div className="border-t border-zinc-700 pt-4">
          <h4 className="text-sm font-medium text-zinc-300 mb-3">Tenant isolation</h4>
          <Badge text={isolation.compliant ? 'compliant' : 'violations'} variant={isolation.compliant ? 'green' : 'red'} />
          {isolation.violations.length > 0 && (
            <ul className="mt-2 space-y-1 text-xs text-red-300/90">
              {isolation.violations.map((v) => (
                <li key={v}>{v}</li>
              ))}
            </ul>
          )}
          {Object.keys(isolation.scheduler_hints).length > 0 && (
            <p className="mt-2 text-xs text-zinc-500 font-mono">
              {Object.entries(isolation.scheduler_hints).map(([k, v]) => `${k}=${v}`).join(', ')}
            </p>
          )}
        </div>
      )}

      {(secretStatus.length > 0 || secretActionMsg) && (
        <div className="border-t border-zinc-700 pt-4">
          <h4 className="text-sm font-medium text-zinc-300 mb-3">Attest-gated secrets</h4>
          {secretActionMsg && (
            <p className="mb-2 text-xs text-zinc-400">{secretActionMsg}</p>
          )}
          {secretStatus.length > 0 ? (
            <div className="space-y-2">
              {secretStatus.map((s) => (
                <div
                  key={s.secret_name}
                  className="flex flex-wrap items-center justify-between gap-2 text-sm py-1.5 px-2 rounded bg-zinc-950/60 border border-zinc-800"
                >
                  <span className="text-zinc-300">{s.secret_name}</span>
                  <div className="flex items-center gap-2">
                    <Badge text={s.state} variant={s.state === 'released' || s.state === 'injected' ? 'green' : s.state === 'revoked' ? 'red' : 'yellow'} />
                    {(s.state === 'pending' || s.state === 'revoked') && (
                      <button
                        type="button"
                        disabled={!canMutate || secretReleaseBusy === s.secret_name}
                        onClick={() => void releaseSecret(s.secret_name, s.provider)}
                        className="rounded border border-zinc-600 px-2 py-0.5 text-[10px] uppercase tracking-wide text-zinc-300 hover:border-aether/50 disabled:opacity-40"
                      >
                        {secretReleaseBusy === s.secret_name ? 'Releasing…' : 'Release'}
                      </button>
                    )}
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <p className="text-xs text-zinc-500">No attest-gated secrets configured on this workload.</p>
          )}
        </div>
      )}
    </div>
  );
}
