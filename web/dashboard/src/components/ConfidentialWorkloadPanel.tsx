import { useEffect, useState } from 'react';
import { apiFetch } from '../utils/api';
import Badge from './Badge';
import type { AttestationExplain, AttestationStatus, AttestGatedSecretStatus, NetworkTrustScore } from '../types/api';

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
}

export default function ConfidentialWorkloadPanel({ workloadName }: ConfidentialWorkloadPanelProps) {
  const [loading, setLoading] = useState(true);
  const [trust, setTrust] = useState<NetworkTrustScore | null>(null);
  const [status, setStatus] = useState<AttestationStatus | null>(null);
  const [explain, setExplain] = useState<AttestationExplain | null>(null);
  const [secretStatus, setSecretStatus] = useState<AttestGatedSecretStatus[]>([]);
  const [showExplain, setShowExplain] = useState(false);
  const [notConfidential, setNotConfidential] = useState(false);

  useEffect(() => {
    let cancelled = false;
    async function load() {
      setLoading(true);
      setNotConfidential(false);
      const [trustData, statusData, secretsData] = await Promise.all([
        apiFetch<NetworkTrustScore>(`/confidential/trust-score/${encodeURIComponent(workloadName)}`),
        apiFetch<AttestationStatus>(`/confidential/attestation/${encodeURIComponent(workloadName)}/status`),
        apiFetch<AttestGatedSecretStatus[]>(`/confidential/secrets/${encodeURIComponent(workloadName)}/status`),
      ]);
      if (cancelled) return;
      if (!trustData) {
        setNotConfidential(true);
      }
      setTrust(trustData);
      setStatus(statusData);
      setSecretStatus(secretsData ?? []);
      setExplain(null);
      setShowExplain(false);
      setLoading(false);
    }
    void load();
    return () => {
      cancelled = true;
    };
  }, [workloadName]);

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
            </div>
          )}
        </div>
      ) : (
        <p className="text-sm text-zinc-500 border-t border-zinc-700 pt-4">
          No attestation record yet. Submit a guest report via Ragnarok or POST{' '}
          <code className="text-zinc-400">/api/confidential/attestation/verify</code>.
        </p>
      )}

      {secretStatus.length > 0 && (
        <div className="border-t border-zinc-700 pt-4">
          <h4 className="text-sm font-medium text-zinc-300 mb-3">Attest-gated secrets</h4>
          <div className="space-y-2">
            {secretStatus.map((s) => (
              <div
                key={s.secret_name}
                className="flex items-center justify-between text-sm py-1.5 px-2 rounded bg-zinc-950/60 border border-zinc-800"
              >
                <span className="text-zinc-300">{s.secret_name}</span>
                <Badge text={s.state} variant={s.state === 'released' || s.state === 'injected' ? 'green' : s.state === 'revoked' ? 'red' : 'yellow'} />
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
