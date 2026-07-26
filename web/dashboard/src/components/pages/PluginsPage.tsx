// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect, useCallback, useMemo } from 'react';
import { Search, Inbox, Plug, Trash2 } from 'lucide-react';
import { Link, useNavigate } from 'react-router';
import { apiFetchSettled, apiPost, apiDelete } from '../../utils/api';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery, useQueryParam, useWorkloadOrSearchFilter } from '../../utils/urlState';
import { useAuth } from '../../contexts/AuthContext';
import PageToolbar from '../PageToolbar';
import CardGrid from '../CardGrid';
import EntityCard from '../EntityCard';
import Badge from '../Badge';
import EmptyState from '../EmptyState';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import Modal from '../Modal';
import { SearchQueryContextBanner, WorkloadScopedCrossLinks } from '../QueryContextBanner';
import type { PluginInfo } from '../../types/api';

export default function PluginsPage() {
  const navigate = useNavigate();
  const [plugins, setPlugins] = useState<PluginInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [discovering, setDiscovering] = useState(false);
  const [discoverSummary, setDiscoverSummary] = useState<string | null>(null);
  const [selectedPlugin, setSelectedPlugin] = useState<PluginInfo | null>(null);
  const [search, setSearch] = useWorkloadOrSearchFilter();
  const [runtimeFilter, setRuntimeFilter] = useQueryParam('runtime', 'all');
  const [registerJson, setRegisterJson] = useState('');
  const [registerMsg, setRegisterMsg] = useState<string | null>(null);
  const { canMutate } = useAuth();

  const load = useCallback(async () => {
    setLoading(true);
    setLoadFailed(false);
    const result = await apiFetchSettled<PluginInfo[]>('/plugins');
    if (!result.ok) {
      setLoadFailed(true);
      setPlugins([]);
    } else {
      setPlugins(result.data);
    }
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function handleDiscover() {
    setDiscovering(true);
    const res = await apiPost<{ discovered?: number; plugins?: string[]; message?: string }>('/plugins/discover');
    setDiscovering(false);
    if (res.success && res.data) {
      const count = res.data.discovered ?? (Array.isArray(res.data.plugins) ? res.data.plugins.length : null);
      setDiscoverSummary(
        count != null ? `Discovered ${count} plugin(s)` : (res.data.message ?? 'Discovery completed')
      );
    } else {
      setDiscoverSummary(res.error ?? 'Discovery failed');
    }
    void load();
  }

  async function handleRegister(e: React.FormEvent) {
    e.preventDefault();
    if (!canMutate || !registerJson.trim()) return;
    try {
      const manifest = JSON.parse(registerJson) as Record<string, unknown>;
      const res = await apiPost('/plugins/register', manifest);
      setRegisterMsg(res.success ? `Registered ${String(manifest.name ?? 'plugin')}` : (res.error ?? 'Failed'));
      if (res.success) void load();
    } catch {
      setRegisterMsg('Invalid JSON manifest');
    }
  }

  async function handleRemove(name: string) {
    if (!canMutate) return;
    const res = await apiDelete(`/plugins/${encodeURIComponent(name)}`);
    setRegisterMsg(res.success ? `Removed ${name}` : (res.error ?? 'Remove failed'));
    void load();
  }

  const runtimeOptions = useMemo(
    () => ['all', ...Array.from(new Set(plugins.map((plugin) => plugin.runtime_kind)))],
    [plugins]
  );

  const visiblePlugins = useMemo(() => {
    const q = search.trim().toLowerCase();
    return plugins.filter((plugin) => {
      if (runtimeFilter !== 'all' && plugin.runtime_kind !== runtimeFilter) return false;
      if (!q) return true;
      return (
        plugin.name.toLowerCase().includes(q) ||
        plugin.runtime_kind.toLowerCase().includes(q) ||
        plugin.capabilities.some((c) => c.toLowerCase().includes(q))
      );
    });
  }, [plugins, runtimeFilter, search]);

  if (loading && plugins.length === 0 && !loadFailed) {
    return <PageLoading rows={4} />;
  }

  if (loadFailed) {
    return <PageLoadError title="Plugins unavailable" onRetry={() => void load()} />;
  }

  return (
    <div>
      <SearchQueryContextBanner testId="plugins-workload-context" query={search} entityLabel="plugins">
        <WorkloadScopedCrossLinks workload={search} prefix="plugins" />
        {search.trim() ? (
          <>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('editor'), { workload: search.trim() })}
              className="text-aether hover:underline"
              data-testid="plugins-editor-link"
            >
              Editor →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('secrets'), { workload: search.trim() })}
              className="text-aether hover:underline"
              data-testid="plugins-secrets-link"
            >
              Secrets →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('drift'), { workload: search.trim() })}
              className="text-aether hover:underline"
              data-testid="plugins-drift-link"
            >
              Drift →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('platform'), { workload: search.trim() })}
              className="text-aether hover:underline"
              data-testid="plugins-context-platform-link"
            >
              Platform →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('gitops'), { workload: search.trim() })}
              className="text-aether hover:underline"
              data-testid="plugins-context-gitops-link"
            >
              GitOps →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('copilot'), { workload: search.trim(), q: `Plugin guidance for ${search.trim()}` })}
              className="text-aether hover:underline"
              data-testid="plugins-context-copilot-link"
            >
              Copilot →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('openapi'), { workload: search.trim() })}
              className="text-aether hover:underline"
              data-testid="plugins-context-openapi-link"
            >
              OpenAPI →
            </Link>
          </>
        ) : null}
      </SearchQueryContextBanner>

      <section className="overview-section-shell mb-6 p-6 sm:p-8">
      <PageToolbar
        search={search}
        onSearchChange={setSearch}
        searchPlaceholder="Search plugins…"
        onRefresh={() => void load()}
        refreshing={loading || discovering}
        filters={
          <select
            value={runtimeFilter}
            onChange={(e) => setRuntimeFilter(e.target.value)}
            className="glass-select"
            aria-label="Runtime filter"
            data-testid="plugins-runtime-filter"
          >
            {runtimeOptions.map((option) => (
              <option key={option} value={option}>
                {option === 'all' ? 'All runtimes' : option}
              </option>
            ))}
          </select>
        }
        actions={
          <button
            type="button"
            data-testid="plugins-discover-button"
            onClick={() => void handleDiscover()}
            disabled={discovering}
            className="inline-flex items-center gap-2 btn-primary disabled:opacity-50"
          >
            <Search size={16} />
            {discovering ? 'Discovering…' : 'Discover'}
          </button>
        }
      />

      <div className="mb-4">
        <button
          type="button"
          data-testid="plugins-scheduler-link"
          onClick={() => navigate(viewToPath('scheduler'))}
          className="text-xs text-aether hover:underline"
        >
          Placement scheduler →
        </button>
      </div>

      {discoverSummary && (
        <div data-testid="plugins-discover-summary" className="glass-panel-card mb-6 text-sm text-slate-300">
          {discoverSummary}
          <button
            type="button"
            onClick={() => navigate(viewToPath('platform'))}
            className="ml-3 text-xs text-aether hover:underline"
          >
            Platform integrations →
          </button>
        </div>
      )}

      {canMutate && (
        <div className="glass-panel-card mb-6">
          <h3 className="text-sm font-semibold text-slate-200 mb-2">Register plugin manifest</h3>
          <form onSubmit={(e) => void handleRegister(e)} className="space-y-3" data-testid="plugins-register-form">
            <textarea
              value={registerJson}
              onChange={(e) => setRegisterJson(e.target.value)}
              rows={4}
              placeholder='{"name":"my-plugin","version":"1.0","runtime_kind":"podman","command":"..."}'
              className="glass-input font-mono text-xs"
            />
            <button type="submit" className="btn-secondary">
              Register
            </button>
          </form>
          {registerMsg && <p className="mt-2 text-xs text-slate-400">{registerMsg}</p>}
        </div>
      )}

      {visiblePlugins.length === 0 ? (
        <EmptyState icon={<Inbox size={48} />} title="No plugins" description="No plugins match your filters. Try discovering plugins." />
      ) : (
        <CardGrid columns="compact" testId="plugins-list">
          {visiblePlugins.map((p, i) => (
            <EntityCard
              key={p.name}
              index={i}
              testId={`plugin-card-${p.name}`}
              icon={<Plug size={18} />}
              statusTone="sky"
              title={p.name}
              subtitle={`${p.runtime_kind} · v${p.version}`}
              onClick={() => setSelectedPlugin(p)}
              body={
                p.capabilities.length > 0 ? (
                  <div className="flex flex-wrap gap-1">
                    {p.capabilities.slice(0, 6).map((c) => (
                      <Badge key={c} text={c} variant="blue" />
                    ))}
                    {p.capabilities.length > 6 ? (
                      <span className="text-[11px] text-slate-500">+{p.capabilities.length - 6}</span>
                    ) : null}
                  </div>
                ) : (
                  <p className="text-[12px] text-slate-500">No declared capabilities</p>
                )
              }
              footer={
                <>
                  <button
                    type="button"
                    onClick={() => setSelectedPlugin(p)}
                    className="inline-flex flex-1 items-center justify-center gap-1.5 rounded-lg px-2 py-1.5 text-[11px] font-medium text-slate-300 transition hover:bg-white/5 hover:text-white"
                  >
                    <Search size={13} />
                    Inspect
                  </button>
                  {canMutate ? (
                    <button
                      type="button"
                      onClick={() => void handleRemove(p.name)}
                      className="inline-flex flex-1 items-center justify-center gap-1.5 rounded-lg px-2 py-1.5 text-[11px] font-medium text-slate-300 transition hover:bg-red-500/15 hover:text-red-300"
                    >
                      <Trash2 size={13} />
                      Remove
                    </button>
                  ) : null}
                </>
              }
            />
          ))}
        </CardGrid>
      )}
      </section>


      <Modal
        isOpen={selectedPlugin !== null}
        onClose={() => setSelectedPlugin(null)}
        title={selectedPlugin ? `Plugin: ${selectedPlugin.name}` : 'Plugin'}
      >
        {selectedPlugin && (
          <div className="space-y-4" data-testid="plugins-inspect-modal">
            <div className="grid grid-cols-1 gap-3 md:grid-cols-3">
              <div className="glass-panel-card px-4 py-3">
                <div className="text-xs uppercase tracking-wider text-slate-500">Runtime</div>
                <div className="mt-2 text-sm font-medium text-slate-100">{selectedPlugin.runtime_kind}</div>
              </div>
              <div className="glass-panel-card px-4 py-3">
                <div className="text-xs uppercase tracking-wider text-slate-500">Version</div>
                <div className="mt-2 text-sm font-medium text-slate-100">{selectedPlugin.version}</div>
              </div>
              <div className="glass-panel-card px-4 py-3">
                <div className="text-xs uppercase tracking-wider text-slate-500">Capabilities</div>
                <div className="mt-2 text-sm font-medium text-slate-100">{selectedPlugin.capabilities.length}</div>
              </div>
            </div>
            <div>
              <div className="mb-2 text-sm font-medium text-slate-200">Command</div>
              <code className="block glass-input text-slate-300 font-mono">
                {selectedPlugin.command}
              </code>
            </div>
            <div className="flex flex-wrap gap-2">
              {selectedPlugin.capabilities.map((capability) => (
                <Badge key={capability} text={capability} variant="blue" />
              ))}
            </div>
          </div>
        )}
      </Modal>
    </div>
  );
}
