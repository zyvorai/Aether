import { useCallback, useEffect, useState } from 'react';
import { Server, Shield, Database, ExternalLink } from 'lucide-react';
import { apiFetch } from '../../utils/api';
import { useServerCapabilities } from '../../contexts/ServerCapabilitiesContext';
import PageToolbar from '../PageToolbar';
import Badge from '../Badge';

interface Integrations {
  backup_remote_configured?: boolean;
  audit_webhook_configured?: boolean;
  grafana_url?: string | null;
  prometheus_url?: string | null;
}

interface ServerPayload {
  version?: string;
  ha_mode?: string;
  embedded_ui_build?: string;
  integrations?: Integrations;
  opa?: { configured?: boolean; enforce?: boolean };
}

export default function PlatformPage() {
  const { capabilities, refreshPlatform } = useServerCapabilities();
  const [server, setServer] = useState<ServerPayload | null>(null);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    setLoading(true);
    const data = await apiFetch<ServerPayload>('/server');
    setServer(data);
    await refreshPlatform();
    setLoading(false);
  }, [refreshPlatform]);

  useEffect(() => {
    void load();
  }, [load]);

  const integrations = server?.integrations ?? {};
  const platform = capabilities?.platform;

  return (
    <div>
      <PageToolbar onRefresh={() => void load()} refreshing={loading} />

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <div className="dash-card">
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
          <p className="mt-4 text-xs text-slate-500">
            Env: <code className="text-slate-400">AETHER_BACKUP_REMOTE_URL</code>,{' '}
            <code className="text-slate-400">AETHER_AUDIT_WEBHOOK_URL</code>,{' '}
            <code className="text-slate-400">AETHER_OPA_URL</code>
          </p>
        </div>
      </div>

      <div className="dash-card mt-6">
        <div className="flex items-center gap-3 mb-4">
          <Database className="text-blue-400" size={20} />
          <h2 className="text-lg font-semibold text-slate-100">Observability links</h2>
        </div>
        <div className="flex flex-wrap gap-3">
          {integrations.grafana_url ? (
            <a
              href={integrations.grafana_url}
              target="_blank"
              rel="noreferrer"
              className="inline-flex items-center gap-2 rounded-xl border border-slate-700 px-4 py-2 text-sm text-slate-200 hover:bg-slate-800"
            >
              Grafana <ExternalLink size={14} />
            </a>
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
              className="inline-flex items-center gap-2 rounded-xl border border-slate-700 px-4 py-2 text-sm text-slate-200 hover:bg-slate-800"
            >
              In-app metrics <ExternalLink size={14} />
            </a>
          )}
        </div>
      </div>
    </div>
  );
}
