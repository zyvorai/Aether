// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from 'react';
import { withAuroraPage } from '../layout/AuroraPage';
import SectionHubPage from '../SectionHubPage';
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
import EcosystemPlatformPanel from '../EcosystemPlatformPanel';
import ProductionTrustPanel from '../ProductionTrustPanel';
import { SectionHeader } from '../layout/SectionHeader';
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

export type PlatformSection = 'recommendations' | 'trust' | 'ecosystem' | 'runtime' | 'cilium' | 'observability';

export function PlatformStudio({ refreshKey, forcedSection }: { refreshKey?: number; forcedSection?: PlatformSection } = {}) {
  const show = (s: PlatformSection) => !forcedSection || forcedSection === s;
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
      // Fire-and-forget: refreshes the shared navbar/context capabilities, but this
      // page's own loading state must not hang on a second, independent network call.
      void refreshPlatform();
    }
    setLoading(false);
    setRecLoading(false);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- refreshKey intentionally triggers a refetch in place, not a remount.
  }, [refreshPlatform, refreshKey]);

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

      <div className="mb-2 flex flex-wrap gap-x-6 gap-y-2 text-sm text-muted" data-testid="platform-hub-context">
        Platform
        {' · '}
        <Link to={viewToPath('fleet')} className="text-primary hover:underline" data-testid="platform-context-fleet-hub-link">
          Fleet →
        </Link>
        {' · '}
        <Link to={viewToPath('hosted')} className="text-primary hover:underline" data-testid="platform-context-hosted-hub-link">
          Hosted SaaS →
        </Link>
        {' · '}
        <Link to={viewToPath('security')} className="text-primary hover:underline" data-testid="platform-context-security-hub-link">
          Security →
        </Link>
        {' · '}
        <Link to={viewToPath('settings')} className="text-primary hover:underline" data-testid="platform-context-settings-hub-link">
          Settings →
        </Link>
      </div>

      {focusedWorkload ? (
        <WorkloadContextBanner testId="platform-workload-context" workload={focusedWorkload} description="Platform context">
          <WorkloadScopedCrossLinks
            workload={focusedWorkload}
            prefix="platform"
            showDrift
            showGitops
            showMetrics
            extraLinks={[
              { label: 'Clusters →', view: 'clusters', testId: 'platform-clusters-link', query: { workload: focusedWorkload } },
              { label: 'RBAC →', view: 'rbac', testId: 'platform-rbac-link', query: { workload: focusedWorkload } },
              { label: 'OpenAPI →', view: 'openapi', testId: 'platform-openapi-link', query: { workload: focusedWorkload } },
              { label: 'Zyra →', view: 'zyra', testId: 'platform-copilot-link', query: { workload: focusedWorkload, q: `Platform guidance for ${focusedWorkload}` } },
              { label: 'Policy →', view: 'policy', testId: 'platform-policy-link', query: { workload: focusedWorkload } },
              { label: 'Intelligence →', view: 'intelligence', testId: 'platform-intelligence-link', query: { workload: focusedWorkload, tab: 'predictions' } },
              { label: 'Secrets →', view: 'secrets', testId: 'platform-secrets-link', query: { workload: focusedWorkload } },
              { label: 'Fleet →', view: 'fleet', testId: 'platform-fleet-link', query: { workload: focusedWorkload } },
              { label: 'Hosted SaaS →', view: 'hosted', testId: 'platform-context-hosted-link', query: {} },
              { label: 'Editor →', view: 'editor', testId: 'platform-editor-link', query: { workload: focusedWorkload } },
              { label: 'Dependencies →', view: 'deps', testId: 'platform-deps-link', query: { workload: focusedWorkload } },
            ]}
          />
        </WorkloadContextBanner>
      ) : null}

      {show('recommendations') ? (
      <div className="mb-6">
        <PlatformRecommendations items={recommendations} loading={recLoading} />
      </div>
      ) : null}

      {show('trust') ? <ProductionTrustPanel /> : null}
      {show('ecosystem') ? <EcosystemPlatformPanel /> : null}

      {show('runtime') ? (
      <section className="apple-chapter space-y-8 mb-10">
        <SectionHeader
          label="Platform"
          title="Runtime & policy"
          description="API server, HA mode, OPA, and integration status"
        />
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <div data-testid="platform-runtime-panel">
          <div className="flex items-center gap-3 mb-4">
            <Server className="text-primary" size={20} />
            <h2 className="text-lg font-semibold text-foreground">Runtime</h2>
          </div>
          <dl className="space-y-3 text-sm">
            <div className="flex justify-between">
              <dt className="text-subtle">API version</dt>
              <dd className="text-foreground font-mono">{server?.version ?? platform?.version ?? '—'}</dd>
            </div>
            <div className="flex justify-between">
              <dt className="text-subtle">HA mode</dt>
              <dd><Badge text={server?.ha_mode ?? platform?.haMode ?? 'single'} variant="blue" /></dd>
            </div>
            <div className="flex justify-between">
              <dt className="text-subtle">System ready</dt>
              <dd>
                <Badge text={systemReady ? 'ready' : 'degraded'} variant={systemReady ? 'green' : 'red'} />
              </dd>
            </div>
            <div className="flex justify-between">
              <dt className="text-subtle">State backend</dt>
              <dd className="text-foreground">{platform?.persistence ?? '—'}</dd>
            </div>
            <div className="flex justify-between">
              <dt className="text-subtle">Embedded UI build</dt>
              <dd className="text-muted font-mono text-xs">{server?.embedded_ui_build ?? '—'}</dd>
            </div>
          </dl>
        </div>

        <div className="glass">
          <div className="flex items-center gap-3 mb-4">
            <Shield className="text-success" size={20} />
            <h2 className="text-lg font-semibold text-foreground">Policy & integrations</h2>
          </div>
          <dl className="space-y-3 text-sm">
            <div className="flex justify-between items-center">
              <dt className="text-subtle">OPA</dt>
              <dd>
                <Badge
                  text={platform?.opa?.configured ? (platform.opa.enforce ? 'enforce' : 'advisory') : 'off'}
                  variant={platform?.opa?.configured ? 'green' : 'muted'}
                />
              </dd>
            </div>
            <div className="flex justify-between">
              <dt className="text-subtle">Remote backup</dt>
              <dd className="text-foreground">{integrations.backup_remote_configured ? 'configured' : 'not set'}</dd>
            </div>
            <div className="flex justify-between">
              <dt className="text-subtle">Audit webhook</dt>
              <dd className="text-foreground">{integrations.audit_webhook_configured ? 'configured' : 'not set'}</dd>
            </div>
          </dl>
          {!platform?.opa?.configured && (
            <Link to={viewToPath('policy')} className="mt-4 inline-flex text-xs text-primary hover:underline">
              Configure policy engine →
            </Link>
          )}
          <Link
            to={
              focusedWorkload
                ? pathWithQuery(viewToPath('audit'), { workload: focusedWorkload })
                : viewToPath('audit')
            }
            className="mt-4 ml-4 inline-flex text-xs text-primary hover:underline"
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
            className="mt-4 ml-4 inline-flex text-xs text-primary hover:underline"
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
            className="mt-4 ml-4 inline-flex text-xs text-primary hover:underline"
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
            className="mt-4 ml-4 inline-flex text-xs text-primary hover:underline"
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
            className="mt-4 ml-4 inline-flex text-xs text-primary hover:underline"
            data-testid="platform-events-link"
          >
            Events feed →
          </Link>
          <p className="mt-4 text-xs text-subtle">
            Env: <code className="text-muted">AETHER_BACKUP_REMOTE_URL</code>,{' '}
            <code className="text-muted">AETHER_AUDIT_WEBHOOK_URL</code>,{' '}
            <code className="text-muted">AETHER_OPA_URL</code>
          </p>
        </div>
        </div>
      </section>

      ) : null}

      {show('cilium') ? (
      <section className="apple-chapter space-y-8 mb-10">
        <SectionHeader
          label="Network"
          title="Kubernetes / Cilium"
          description="CNI status, connectivity checks, and bootstrap policies"
        />
      <div className="glass">
        <div className="flex items-center gap-3 mb-4">
          <Network className="text-lavender" size={20} />
          <h2 className="text-lg font-semibold text-foreground">Kubernetes / Cilium</h2>
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
              <dt className="text-subtle">Cluster</dt>
              <dd className="text-foreground font-mono text-xs">{cilium.cluster}</dd>
            </div>
            <div className="flex justify-between">
              <dt className="text-subtle">Egress mode</dt>
              <dd><Badge text={cilium.egress_mode} variant={cilium.egress_mode === 'strict' ? 'yellow' : 'blue'} /></dd>
            </div>
            <div className="flex justify-between">
              <dt className="text-subtle">Cilium agent</dt>
              <dd><Badge text={cilium.cilium_daemonset_ready ? 'ready' : 'not ready'} variant={cilium.cilium_daemonset_ready ? 'green' : 'red'} /></dd>
            </div>
            <div className="flex justify-between">
              <dt className="text-subtle">metrics-server</dt>
              <dd><Badge text={cilium.metrics_server ? 'available' : 'missing'} variant={cilium.metrics_server ? 'green' : 'muted'} /></dd>
            </div>
            <div className="flex justify-between items-center">
              <dt className="text-subtle">Connectivity check</dt>
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
                    className="text-xs text-primary hover:underline disabled:opacity-50"
                    data-testid="platform-cilium-probe-btn"
                  >
                    {probeLoading ? 'Running…' : 'Run check'}
                  </button>
                )}
              </dd>
            </div>
            {cilium.last_checked_at && (
              <div className="flex justify-between">
                <dt className="text-subtle">Last checked</dt>
                <dd className="text-muted text-xs font-mono">{cilium.last_checked_at}</dd>
              </div>
            )}
            {cilium.connectivity_detail && (
              <div className="flex justify-between gap-4">
                <dt className="text-subtle shrink-0">Detail</dt>
                <dd className="text-muted text-xs text-right">{cilium.connectivity_detail}</dd>
              </div>
            )}
            <div>
              <dt className="text-subtle mb-2">Bootstrap policies</dt>
              <dd className="space-y-1">
                {cilium.managed_policies.map((policy) => (
                  <div key={policy.name} className="flex items-center justify-between rounded-lg border glass-divider px-3 py-2">
                    <span className="font-mono text-xs text-muted">{policy.name}</span>
                    <Badge text={policy.exists ? 'applied' : 'missing'} variant={policy.exists ? 'green' : 'red'} />
                  </div>
                ))}
              </dd>
            </div>
            {cilium.cluster && (
              <Link
                to={pathWithQuery(viewToPath('clusters'), { cluster: cilium.cluster, tab: 'network' })}
                className="inline-flex text-xs text-primary hover:underline"
                data-testid="platform-cilium-cluster-link"
              >
                Browse network policies in cluster browser →
              </Link>
            )}
          </dl>
        ) : (
          <p className="text-sm text-subtle">Cilium status unavailable — ensure kubeconfig is reachable from the API server.</p>
        )}
        <p className="mt-4 text-xs text-subtle">
          Browse Cilium policies on the Cluster Browser <strong>Network</strong> tab. Hubble UI is auto-discovered when installed, or set{' '}
          <code className="text-muted">AETHER_HUBBLE_UI_URL</code>.
        </p>
      </div>
      </section>

      ) : null}

      {show('observability') ? (
      <section className="glass p-6 sm:p-8">
        <SectionHeader
          label="Observability"
          title="External links"
          description="Grafana, Prometheus, Hubble, and PacketWolf integrations"
        />
      <div className="glass" data-testid="platform-observability-panel">
        <div className="flex items-center gap-3 mb-4">
          <Database className="text-primary" size={20} />
          <h2 className="text-lg font-semibold text-foreground">Observability links</h2>
        </div>
        <div className="flex flex-wrap gap-3">
          {integrations.grafana_url ? (
            <>
              <a
                href={integrations.grafana_url}
                target="_blank"
                rel="noreferrer"
                className="inline-flex items-center gap-2 rounded-xl border glass-divider px-4 py-2 text-sm text-foreground glass-inset-hover"
              >
                Grafana <ExternalLink size={14} />
              </a>
              {grafanaDashboardUrl ? (
                <a
                  href={grafanaDashboardUrl}
                  target="_blank"
                  rel="noreferrer"
                  className="inline-flex items-center gap-2 rounded-xl border border-primary/40 bg-primary/10 px-4 py-2 text-sm text-primary hover:bg-primary/20"
                >
                  Open Aether dashboard <ExternalLink size={14} />
                </a>
              ) : integrations.prometheus_url ? (
                <span className="text-sm text-subtle self-center">
                  Set <code>AETHER_GRAFANA_DASHBOARD_UID</code> and run <code>scripts/import-grafana-dashboard.sh</code>
                </span>
              ) : null}
            </>
          ) : (
            <span className="text-sm text-subtle">Set <code>AETHER_GRAFANA_URL</code> for Grafana link</span>
          )}
          {integrations.prometheus_url ? (
            <a
              href={integrations.prometheus_url}
              target="_blank"
              rel="noreferrer"
              className="inline-flex items-center gap-2 rounded-xl border glass-divider px-4 py-2 text-sm text-foreground glass-inset-hover"
            >
              Prometheus <ExternalLink size={14} />
            </a>
          ) : (
            <a
              href="/metrics"
              data-testid="platform-metrics-link"
              className="inline-flex items-center gap-2 rounded-xl border glass-divider px-4 py-2 text-sm text-foreground glass-inset-hover"
            >
              In-app metrics <ExternalLink size={14} />
            </a>
          )}
          {integrations.hubble_ui_url && (
            <a
              href={integrations.hubble_ui_url}
              target="_blank"
              rel="noreferrer"
              className="inline-flex items-center gap-2 rounded-xl border border-lavender/50 bg-lavender/30 px-4 py-2 text-sm text-lavender hover:bg-lavender/50"
            >
              Hubble UI <ExternalLink size={14} />
            </a>
          )}
          {integrations.packetwolf_url && (
            <a
              href={integrations.packetwolf_url}
              target="_blank"
              rel="noreferrer"
              className="inline-flex items-center gap-2 rounded-xl border border-primary/50 bg-primary/30 px-4 py-2 text-sm text-primary hover:bg-primary/50"
            >
              PacketWolf <ExternalLink size={14} />
            </a>
          )}
          {!integrations.hubble_ui_url && (
            <span className="text-sm text-subtle self-center">
              Set <code className="text-muted">AETHER_HUBBLE_UI_URL</code> for Hubble
            </span>
          )}
        </div>
        <p className="mt-4 text-xs text-subtle">
          Multi-cluster inventory: open{' '}
          <a href="/fleet" className="text-primary hover:underline">
            Fleet Overview
          </a>
          . Set <code className="text-muted">AETHER_PACKETWOLF_URL</code> for PacketWolf east-west verification.
        </p>
      </div>
      </section>
      ) : null}
    </div>
  );
}

function PlatformHubPage() {
  return (
    <SectionHubPage
      links={[
        { view: 'platform-recommendations', title: 'Recommendations', description: 'Setup and HA recommendations.', icon: <Server className="h-5 w-5" /> },
        { view: 'platform-trust', title: 'Production trust', description: 'Trust and readiness signals.', icon: <Shield className="h-5 w-5" /> },
        { view: 'platform-ecosystem', title: 'Ecosystem', description: 'Ecosystem integrations.', icon: <Network className="h-5 w-5" /> },
        { view: 'platform-runtime', title: 'Runtime & policy', description: 'API server, HA, OPA, integrations.', icon: <Server className="h-5 w-5" /> },
        { view: 'platform-cilium', title: 'Kubernetes / Cilium', description: 'CNI status and connectivity.', icon: <Network className="h-5 w-5" /> },
        { view: 'platform-observability', title: 'Observability links', description: 'Grafana, Prometheus, Hubble, PacketWolf.', icon: <Database className="h-5 w-5" /> },
      ]}
    />
  );
}

export default withAuroraPage('platform', PlatformHubPage);

