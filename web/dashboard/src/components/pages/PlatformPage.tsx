// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from 'react';
import { Link } from 'react-router';
import { Server, Shield, Database, ExternalLink, Network } from 'lucide-react';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery, useQueryParam } from '../../utils/urlState';
import { WorkloadContextBanner, WorkloadScopedCrossLinks } from '../QueryContextBanner';
import { apiFetchSettled, apiPost } from '../../utils/api';
import { useServerCapabilities } from '../../contexts/ServerCapabilitiesContext';
import PageToolbar from '../PageToolbar';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import Badge from '../Badge';
import PlatformRecommendations from '../PlatformRecommendations';
import type { CiliumStatusResponse, PlatformRecommendation } from '../../types/api';

interface Integrations {
  backup_remote_configured?: boolean;
  audit_webhook_configured?: boolean;
  grafana_url?: string | null;
  prometheus_url?: string | null;
  hubble_ui_url?: string | null;
  grafana_dashboard_uid?: string | null;
  packetwolf_url?: string | null;
}

interface ServerPayload {
  version?: string;
  ha_mode?: string;
  embedded_ui_build?: string;
  integrations?: Integrations;
  kubernetes?: { cilium?: CiliumStatusResponse | null };
  opa?: { configured?: boolean; enforce?: boolean };
}

export default function PlatformPage() {
  const [workloadFocus] = useQueryParam('workload');
  const focusedWorkload = workloadFocus.trim();
  const { capabilities, ready, refreshPlatform } = useServerCapabilities();
  const [server, setServer] = useState<ServerPayload | null>(null);
  const [recommendations, setRecommendations] = useState<PlatformRecommendation[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [recLoading, setRecLoading] = useState(true);
  const [probeLoading, setProbeLoading] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    setRecLoading(true);
    setLoadFailed(false);
    const [serverRes, recRes] = await Promise.all([
      apiFetchSettled<ServerPayload>('/server'),
      apiFetchSettled<{ items?: PlatformRecommendation[] }>('/platform/recommendations'),
    ]);
    if (!serverRes.ok && !recRes.ok) {
      setLoadFailed(true);
      setServer(null);
      setRecommendations([]);
    } else {
      setServer(serverRes.ok ? serverRes.data : null);
      setRecommendations(recRes.ok ? (recRes.data.items ?? []) : []);
      await refreshPlatform();
    }
    setLoading(false);
    setRecLoading(false);
  }, [refreshPlatform]);

  const runConnectivityProbe = useCallback(async () => {
    setProbeLoading(true);
    const res = await apiPost<{ status?: string; detail?: string; checked_at?: string }>(
      '/cluster/cilium/connectivity/probe',
      {},
    );
    setProbeLoading(false);
    if (res.success) {
      await load();
    }
  }, [load]);

  useEffect(() => {
    void load();
  }, [load]);

  const platform = capabilities?.platform;
  const integrations = server?.integrations ?? {};
  const cilium = server?.kubernetes?.cilium ?? platform?.kubernetes?.cilium ?? null;
  const systemReady = ready?.ready ?? true;
  const grafanaDashboardUrl =
    integrations.grafana_url && integrations.grafana_dashboard_uid
      ? `${integrations.grafana_url.replace(/\/$/, '')}/d/${integrations.grafana_dashboard_uid}`
      : null;

  if (loading && !server && !loadFailed) {
    return <PageLoading rows={5} />;
  }

  if (loadFailed) {
    return <PageLoadError title="Platform data unavailable" onRetry={() => void load()} />;
  }

  return (
    <div>
      <PageToolbar
        onRefresh={() => void load()}
        refreshing={loading}
        refreshTestId="platform-toolbar-refresh"
      />

      {focusedWorkload ? (
        <WorkloadContextBanner testId="platform-workload-context" workload={focusedWorkload} description="Platform context">
          <WorkloadScopedCrossLinks
            workload={focusedWorkload}
            prefix="platform"
            showDrift
            showGitops
            showMetrics
          />
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('clusters'), { workload: focusedWorkload })}
            className="text-aether hover:underline"
            data-testid="platform-clusters-link"
          >
            Clusters →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('rbac'), { workload: focusedWorkload })}
            className="text-aether hover:underline"
            data-testid="platform-rbac-link"
          >
            RBAC →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('openapi'), { workload: focusedWorkload })}
            className="text-aether hover:underline"
            data-testid="platform-openapi-link"
          >
            OpenAPI →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('copilot'), { workload: focusedWorkload, q: `Platform guidance for ${focusedWorkload}` })}
            className="text-aether hover:underline"
            data-testid="platform-copilot-link"
          >
            Copilot →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('policy'), { workload: focusedWorkload })}
            className="text-aether hover:underline"
            data-testid="platform-policy-link"
          >
            Policy →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('intelligence'), { workload: focusedWorkload, tab: 'predictions' })}
            className="text-aether hover:underline"
            data-testid="platform-intelligence-link"
          >
            Intelligence →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('secrets'), { workload: focusedWorkload })}
            className="text-aether hover:underline"
            data-testid="platform-secrets-link"
          >
            Secrets →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('fleet'), { workload: focusedWorkload })}
            className="text-aether hover:underline"
            data-testid="platform-fleet-link"
          >
            Fleet →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('editor'), { workload: focusedWorkload })}
            className="text-aether hover:underline"
            data-testid="platform-editor-link"
          >
            Editor →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('deps'), { workload: focusedWorkload })}
            className="text-aether hover:underline"
            data-testid="platform-deps-link"
          >
            Dependencies →
          </Link>
        </WorkloadContextBanner>
      ) : null}

      <div className="mb-6">
        <PlatformRecommendations items={recommendations} loading={recLoading} />
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <div className="dash-card" data-testid="platform-runtime-panel">
          <div className="flex items-center gap-3 mb-4">
            <Server className="text-aether" size={20} />
            <h2 className="text-lg font-semibold text-slate-100">Runtime</h2>
          </div>
          <dl className="space-y-3 text-sm">
            <div className="flex justify-between">
              <dt className="text-slate-500">API version</dt>
              <dd className="text-slate-200 font-mono">{server?.version ?? platform?.version ?? '—'}</dd>
            </div>
            <div className="flex justify-between">
              <dt className="text-slate-500">HA mode</dt>
              <dd><Badge text={server?.ha_mode ?? platform?.haMode ?? 'single'} variant="blue" /></dd>
            </div>
            <div className="flex justify-between">
              <dt className="text-slate-500">System ready</dt>
              <dd>
                <Badge text={systemReady ? 'ready' : 'degraded'} variant={systemReady ? 'green' : 'red'} />
              </dd>
            </div>
            <div className="flex justify-between">
              <dt className="text-slate-500">State backend</dt>
              <dd className="text-slate-200">{platform?.persistence ?? '—'}</dd>
            </div>
            <div className="flex justify-between">
              <dt className="text-slate-500">Embedded UI build</dt>
              <dd className="text-slate-400 font-mono text-xs">{server?.embedded_ui_build ?? '—'}</dd>
            </div>
          </dl>
        </div>

        <div className="dash-card">
          <div className="flex items-center gap-3 mb-4">
            <Shield className="text-emerald-400" size={20} />
            <h2 className="text-lg font-semibold text-slate-100">Policy & integrations</h2>
          </div>
          <dl className="space-y-3 text-sm">
            <div className="flex justify-between items-center">
              <dt className="text-slate-500">OPA</dt>
              <dd>
                <Badge
                  text={platform?.opa?.configured ? (platform.opa.enforce ? 'enforce' : 'advisory') : 'off'}
                  variant={platform?.opa?.configured ? 'green' : 'muted'}
                />
              </dd>
            </div>
            <div className="flex justify-between">
              <dt className="text-slate-500">Remote backup</dt>
              <dd className="text-slate-200">{integrations.backup_remote_configured ? 'configured' : 'not set'}</dd>
            </div>
            <div className="flex justify-between">
              <dt className="text-slate-500">Audit webhook</dt>
              <dd className="text-slate-200">{integrations.audit_webhook_configured ? 'configured' : 'not set'}</dd>
            </div>
          </dl>
          {!platform?.opa?.configured && (
            <Link to={viewToPath('policy')} className="mt-4 inline-flex text-xs text-aether hover:underline">
              Configure policy engine →
            </Link>
          )}
          <Link
            to={
              focusedWorkload
                ? pathWithQuery(viewToPath('audit'), { workload: focusedWorkload })
                : viewToPath('audit')
            }
            className="mt-4 ml-4 inline-flex text-xs text-aether hover:underline"
            data-testid="platform-audit-link"
          >
            Audit trail →
          </Link>
          <Link
            to={
              focusedWorkload
                ? pathWithQuery(viewToPath('backups'), { workload: focusedWorkload })
                : viewToPath('backups')
            }
            className="mt-4 ml-4 inline-flex text-xs text-aether hover:underline"
            data-testid="platform-backups-link"
          >
            Backups →
          </Link>
          <Link
            to={
              focusedWorkload
                ? pathWithQuery(viewToPath('confidential'), { workload: focusedWorkload })
                : viewToPath('confidential')
            }
            className="mt-4 ml-4 inline-flex text-xs text-aether hover:underline"
            data-testid="platform-confidential-link"
          >
            Confidential computing →
          </Link>
          <Link
            to={
              focusedWorkload
                ? pathWithQuery(viewToPath('health'), { workload: focusedWorkload })
                : viewToPath('health')
            }
            className="mt-4 ml-4 inline-flex text-xs text-aether hover:underline"
            data-testid="platform-health-link"
          >
            Health monitor →
          </Link>
          <Link
            to={
              focusedWorkload
                ? pathWithQuery(viewToPath('events'), { workload: focusedWorkload })
                : viewToPath('events')
            }
            className="mt-4 ml-4 inline-flex text-xs text-aether hover:underline"
            data-testid="platform-events-link"
          >
            Events feed →
          </Link>
          <p className="mt-4 text-xs text-slate-500">
            Env: <code className="text-slate-400">AETHER_BACKUP_REMOTE_URL</code>,{' '}
            <code className="text-slate-400">AETHER_AUDIT_WEBHOOK_URL</code>,{' '}
            <code className="text-slate-400">AETHER_OPA_URL</code>
          </p>
        </div>
      </div>

      <div className="dash-card mt-6">
        <div className="flex items-center gap-3 mb-4">
          <Network className="text-purple-400" size={20} />
          <h2 className="text-lg font-semibold text-slate-100">Kubernetes / Cilium</h2>
          {cilium && (
            <Badge
              text={cilium.cni === 'cilium' ? 'Cilium' : cilium.cni}
              variant={cilium.cni === 'cilium' ? 'green' : cilium.cni === 'unknown' ? 'muted' : 'yellow'}
            />
          )}
        </div>
        {cilium ? (
          <dl className="space-y-3 text-sm">
            <div className="flex justify-between">
              <dt className="text-slate-500">Cluster</dt>
              <dd className="text-slate-200 font-mono text-xs">{cilium.cluster}</dd>
            </div>
            <div className="flex justify-between">
              <dt className="text-slate-500">Egress mode</dt>
              <dd><Badge text={cilium.egress_mode} variant={cilium.egress_mode === 'strict' ? 'yellow' : 'blue'} /></dd>
            </div>
            <div className="flex justify-between">
              <dt className="text-slate-500">Cilium agent</dt>
              <dd><Badge text={cilium.cilium_daemonset_ready ? 'ready' : 'not ready'} variant={cilium.cilium_daemonset_ready ? 'green' : 'red'} /></dd>
            </div>
            <div className="flex justify-between">
              <dt className="text-slate-500">metrics-server</dt>
              <dd><Badge text={cilium.metrics_server ? 'available' : 'missing'} variant={cilium.metrics_server ? 'green' : 'muted'} /></dd>
            </div>
            <div className="flex justify-between items-center">
              <dt className="text-slate-500">Connectivity check</dt>
              <dd className="flex items-center gap-2">
                <Badge
                  text={cilium.connectivity_check ?? 'unknown'}
                  variant={
                    cilium.connectivity_check === 'ok'
                      ? 'green'
                      : cilium.connectivity_check === 'failed'
                        ? 'red'
                        : 'muted'
                  }
                />
                {cilium.cni === 'cilium' && (
                  <button
                    type="button"
                    onClick={() => void runConnectivityProbe()}
                    disabled={probeLoading}
                    className="text-xs text-aether hover:underline disabled:opacity-50"
                    data-testid="platform-cilium-probe-btn"
                  >
                    {probeLoading ? 'Running…' : 'Run check'}
                  </button>
                )}
              </dd>
            </div>
            {cilium.last_checked_at && (
              <div className="flex justify-between">
                <dt className="text-slate-500">Last checked</dt>
                <dd className="text-slate-400 text-xs font-mono">{cilium.last_checked_at}</dd>
              </div>
            )}
            {cilium.connectivity_detail && (
              <div className="flex justify-between gap-4">
                <dt className="text-slate-500 shrink-0">Detail</dt>
                <dd className="text-slate-400 text-xs text-right">{cilium.connectivity_detail}</dd>
              </div>
            )}
            <div>
              <dt className="text-slate-500 mb-2">Bootstrap policies</dt>
              <dd className="space-y-1">
                {cilium.managed_policies.map((policy) => (
                  <div key={policy.name} className="flex items-center justify-between rounded-lg border border-slate-800 px-3 py-2">
                    <span className="font-mono text-xs text-slate-300">{policy.name}</span>
                    <Badge text={policy.exists ? 'applied' : 'missing'} variant={policy.exists ? 'green' : 'red'} />
                  </div>
                ))}
              </dd>
            </div>
            {cilium.cluster && (
              <Link
                to={pathWithQuery(viewToPath('clusters'), { cluster: cilium.cluster, tab: 'network' })}
                className="inline-flex text-xs text-aether hover:underline"
                data-testid="platform-cilium-cluster-link"
              >
                Browse network policies in cluster browser →
              </Link>
            )}
          </dl>
        ) : (
          <p className="text-sm text-slate-500">Cilium status unavailable — ensure kubeconfig is reachable from the API server.</p>
        )}
        <p className="mt-4 text-xs text-slate-500">
          Browse Cilium policies on the Cluster Browser <strong>Network</strong> tab. Hubble UI is auto-discovered when installed, or set{' '}
          <code className="text-slate-400">AETHER_HUBBLE_UI_URL</code>.
        </p>
      </div>

      <div className="dash-card mt-6" data-testid="platform-observability-panel">
        <div className="flex items-center gap-3 mb-4">
          <Database className="text-blue-400" size={20} />
          <h2 className="text-lg font-semibold text-slate-100">Observability links</h2>
        </div>
        <div className="flex flex-wrap gap-3">
          {integrations.grafana_url ? (
            <>
              <a
                href={integrations.grafana_url}
                target="_blank"
                rel="noreferrer"
                className="inline-flex items-center gap-2 rounded-xl border border-slate-700 px-4 py-2 text-sm text-slate-200 hover:bg-slate-800"
              >
                Grafana <ExternalLink size={14} />
              </a>
              {grafanaDashboardUrl ? (
                <a
                  href={grafanaDashboardUrl}
                  target="_blank"
                  rel="noreferrer"
                  className="inline-flex items-center gap-2 rounded-xl border border-aether/40 bg-aether/10 px-4 py-2 text-sm text-aether hover:bg-aether/20"
                >
                  Open Aether dashboard <ExternalLink size={14} />
                </a>
              ) : integrations.prometheus_url ? (
                <span className="text-sm text-slate-500 self-center">
                  Set <code>AETHER_GRAFANA_DASHBOARD_UID</code> and run <code>scripts/import-grafana-dashboard.sh</code>
                </span>
              ) : null}
            </>
          ) : (
            <span className="text-sm text-slate-500">Set <code>AETHER_GRAFANA_URL</code> for Grafana link</span>
          )}
          {integrations.prometheus_url ? (
            <a
              href={integrations.prometheus_url}
              target="_blank"
              rel="noreferrer"
              className="inline-flex items-center gap-2 rounded-xl border border-slate-700 px-4 py-2 text-sm text-slate-200 hover:bg-slate-800"
            >
              Prometheus <ExternalLink size={14} />
            </a>
          ) : (
            <a
              href="/metrics"
              data-testid="platform-metrics-link"
              className="inline-flex items-center gap-2 rounded-xl border border-slate-700 px-4 py-2 text-sm text-slate-200 hover:bg-slate-800"
            >
              In-app metrics <ExternalLink size={14} />
            </a>
          )}
          {integrations.hubble_ui_url && (
            <a
              href={integrations.hubble_ui_url}
              target="_blank"
              rel="noreferrer"
              className="inline-flex items-center gap-2 rounded-xl border border-purple-700/50 bg-purple-950/30 px-4 py-2 text-sm text-purple-200 hover:bg-purple-950/50"
            >
              Hubble UI <ExternalLink size={14} />
            </a>
          )}
          {integrations.packetwolf_url && (
            <a
              href={integrations.packetwolf_url}
              target="_blank"
              rel="noreferrer"
              className="inline-flex items-center gap-2 rounded-xl border border-cyan-700/50 bg-cyan-950/30 px-4 py-2 text-sm text-cyan-200 hover:bg-cyan-950/50"
            >
              PacketWolf <ExternalLink size={14} />
            </a>
          )}
          {!integrations.hubble_ui_url && (
            <span className="text-sm text-slate-500 self-center">
              Set <code className="text-slate-400">AETHER_HUBBLE_UI_URL</code> for Hubble
            </span>
          )}
        </div>
        <p className="mt-4 text-xs text-slate-500">
          Multi-cluster inventory: open{' '}
          <a href="/fleet" className="text-aether hover:underline">
            Fleet Overview
          </a>
          . Set <code className="text-slate-400">AETHER_PACKETWOLF_URL</code> for PacketWolf east-west verification.
        </p>
      </div>
    </div>
  );
}
