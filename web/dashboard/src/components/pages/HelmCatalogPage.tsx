// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useMemo, useState } from 'react';
import { withAuroraPage } from '../layout/AuroraPage';
import { Database, Download, Package, Search, Shield, Wrench } from 'lucide-react';
import { apiFetchSettled, apiPost } from '../../utils/api';
import PageToolbar from '../PageToolbar';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import Modal from '../Modal';
import type { ClusterSummary, HelmCatalogChart } from '../../types/api';
import { useAuth } from '../../contexts/AuthContext';
import {
  buildValuesYaml,
  defaultFormValues,
  helmFormFields,
  type HelmFormField,
} from '../../utils/helmValuesForms';

const CATEGORY_ICONS: Record<string, typeof Database> = {
  Databases: Database,
  Observability: Package,
  Security: Shield,
  DevOps: Wrench,
  Networking: Package,
};

function HelmCatalogPage({ refreshKey }: { refreshKey?: number } = {}) {
  const { canMutate } = useAuth();
  const [charts, setCharts] = useState<HelmCatalogChart[]>([]);
  const [clusters, setClusters] = useState<ClusterSummary | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [hasLoadedOnce, setHasLoadedOnce] = useState(false);
  const [search, setSearch] = useState('');
  const [category, setCategory] = useState('all');
  const [selected, setSelected] = useState<HelmCatalogChart | null>(null);
  const [installCluster, setInstallCluster] = useState('');
  const [releaseName, setReleaseName] = useState('');
  const [namespace, setNamespace] = useState('default');
  const [formValues, setFormValues] = useState<Record<string, string | boolean>>({});
  const [showAdvancedYaml, setShowAdvancedYaml] = useState(false);
  const [valuesYaml, setValuesYaml] = useState('');
  const [installing, setInstalling] = useState(false);
  const [installMsg, setInstallMsg] = useState<string | null>(null);

  const formFields: HelmFormField[] = useMemo(
    () => (selected ? helmFormFields(selected.id) : []),
    [selected],
  );

  const generatedYaml = useMemo(() => {
    if (!selected) return '';
    return buildValuesYaml(selected.id, formValues);
  }, [selected, formValues]);

  const load = useCallback(async () => {
    setLoading(true);
    setLoadFailed(false);
    const [catalogRes, clusterRes] = await Promise.all([
      apiFetchSettled<HelmCatalogChart[]>('/helm/catalog'),
      apiFetchSettled<ClusterSummary>('/cluster/summary'),
    ]);
    if (!catalogRes.ok) {
      setLoadFailed(true);
      setLoading(false);
      setHasLoadedOnce(true);
      return;
    }
    setCharts(catalogRes.data);
    setClusters(clusterRes.ok ? clusterRes.data : null);
    if (clusterRes.ok && clusterRes.data.clusters[0]) {
      setInstallCluster(clusterRes.data.clusters[0].name);
    }
    setLoading(false);
    setHasLoadedOnce(true);
  }, [refreshKey]);

  useEffect(() => {
    void load();
  }, [load]);

  const categories = useMemo(() => ['all', ...Array.from(new Set(charts.map((c) => c.category))).sort()], [charts]);

  const filtered = useMemo(() => {
    const q = search.trim().toLowerCase();
    return charts.filter((c) => {
      if (category !== 'all' && c.category !== category) return false;
      if (!q) return true;
      return c.name.toLowerCase().includes(q) || c.description.toLowerCase().includes(q);
    });
  }, [charts, search, category]);

  function openInstall(chart: HelmCatalogChart) {
    setSelected(chart);
    setReleaseName(chart.id);
    setFormValues(defaultFormValues(chart.id));
    setShowAdvancedYaml(false);
    setValuesYaml('');
    setInstallMsg(null);
  }

  function setField(key: string, value: string | boolean) {
    setFormValues((prev) => ({ ...prev, [key]: value }));
  }

  async function runInstall() {
    if (!selected || !canMutate) return;
    setInstalling(true);
    setInstallMsg(null);
    const yaml = showAdvancedYaml && valuesYaml.trim() ? valuesYaml : generatedYaml;
    const res = await apiPost<string>('/cluster/helm/action', {
      cluster: installCluster,
      namespace,
      release: releaseName,
      action: 'install',
      chart: selected.chart,
      repo: selected.repo,
      values_yaml: yaml || undefined,
    });
    setInstalling(false);
    setInstallMsg(res.success ? (res.data ?? 'Install triggered') : (res.error ?? 'Install failed'));
  }

  if (loading && !hasLoadedOnce) return <PageLoading label="Loading Helm catalog…" />;
  if (loadFailed) return <PageLoadError title="Helm catalog unavailable" onRetry={() => void load()} />;

  return (
    <div data-testid="helm-catalog-page">
      <PageToolbar onRefresh={() => void load()} refreshing={loading} />

      <section>
      <div className="flex flex-col sm:flex-row gap-3 mb-6">
        <div className="relative flex-1">
          <Search size={16} className="absolute left-3 top-1/2 -translate-y-1/2 text-ink-3" />
          <input
            type="search"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Search charts…"
            className="glass-input pl-9"
          />
        </div>
        <div className="flex flex-wrap gap-2">
          {categories.map((cat) => (
            <button
              key={cat}
              type="button"
              onClick={() => setCategory(cat)}
              className={`glass-tab tab-chip capitalize ${category === cat ? 'glass-tab-active tab-chip-active' : ''}`}
            >
              {cat === 'all' ? 'All' : cat}
            </button>
          ))}
        </div>
      </div>

      <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-3 gap-4">
        {filtered.map((chart) => {
          const Icon = CATEGORY_ICONS[chart.category] ?? Package;
          return (
            <article
              key={chart.id}
              className="glass p-5 hover:border-brand/40 transition-colors"
            >
              <div className="flex items-start gap-3">
                <div className="rounded-xl bg-brand/10 p-3 text-brand">
                  <Icon size={22} />
                </div>
                <div className="min-w-0 flex-1">
                  <h3 className="font-semibold text-ink">{chart.name}</h3>
                  <p className="text-xs text-ink-3">{chart.category} · v{chart.version}</p>
                </div>
              </div>
              <p className="text-sm text-ink-2 mt-3 line-clamp-2">{chart.description}</p>
              <div className="mt-3 flex flex-wrap gap-2 text-[10px] text-ink-3">
                {chart.storage_required && <span className="rounded glass-inset-surface px-2 py-0.5">Storage</span>}
                {chart.ha_available && <span className="rounded glass-inset-surface px-2 py-0.5">HA</span>}
                {chart.backup_supported && <span className="rounded glass-inset-surface px-2 py-0.5">Backup</span>}
                {chart.monitoring_available && <span className="rounded glass-inset-surface px-2 py-0.5">Monitoring</span>}
              </div>
              <button
                type="button"
                onClick={() => openInstall(chart)}
                disabled={!canMutate}
                className="mt-4 w-full inline-flex items-center justify-center gap-2 btn-primary disabled:opacity-50"
              >
                <Download size={16} /> Install
              </button>
            </article>
          );
        })}
      </div>
      </section>

      <Modal isOpen={Boolean(selected)} onClose={() => setSelected(null)} title={selected ? `Install ${selected.name}` : ''}>
        {selected && (
          <div className="space-y-4">
            <p className="text-sm text-ink-2">{selected.description}</p>
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
              <label className="block text-xs text-ink-3">
                Cluster
                <select
                  value={installCluster}
                  onChange={(e) => setInstallCluster(e.target.value)}
                  className="glass-input mt-1"
                >
                  {(clusters?.clusters ?? []).map((c) => (
                    <option key={c.name} value={c.name}>{c.name}</option>
                  ))}
                </select>
              </label>
              <label className="block text-xs text-ink-3">
                Namespace (workspace)
                <input
                  value={namespace}
                  onChange={(e) => setNamespace(e.target.value)}
                  className="glass-input mt-1"
                />
              </label>
              <label className="block text-xs text-ink-3">
                Release name
                <input
                  value={releaseName}
                  onChange={(e) => setReleaseName(e.target.value)}
                  className="glass-input mt-1"
                />
              </label>
            </div>

            {!showAdvancedYaml ? (
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                {formFields.map((field) => (
                  <label key={field.key} className="block text-xs text-ink-3">
                    {field.label}
                    {field.type === 'boolean' ? (
                      <input
                        type="checkbox"
                        checked={formValues[field.key] === true}
                        onChange={(e) => setField(field.key, e.target.checked)}
                        className="mt-2 accent-primary"
                      />
                    ) : field.type === 'select' && field.options ? (
                      <select
                        value={String(formValues[field.key] ?? '')}
                        onChange={(e) => setField(field.key, e.target.value)}
                        className="glass-input mt-1"
                      >
                        {field.options.map((opt) => (
                          <option key={opt.value} value={opt.value}>{opt.label}</option>
                        ))}
                      </select>
                    ) : (
                      <input
                        type={field.type === 'password' ? 'password' : field.type === 'number' ? 'number' : 'text'}
                        value={String(formValues[field.key] ?? '')}
                        placeholder={field.placeholder}
                        onChange={(e) => setField(field.key, e.target.value)}
                        className="glass-input mt-1"
                      />
                    )}
                    {field.help && <span className="block mt-1 text-[10px] text-ink-3">{field.help}</span>}
                  </label>
                ))}
              </div>
            ) : null}

            <div className="flex items-center justify-between gap-2">
              <label className="flex items-center gap-2 text-xs text-ink-2">
                <input
                  type="checkbox"
                  checked={showAdvancedYaml}
                  onChange={(e) => setShowAdvancedYaml(e.target.checked)}
                  className="accent-primary"
                />
                Edit raw values.yaml
              </label>
              {!showAdvancedYaml && (
                <span className="text-[10px] text-ink-3">Generated from form above</span>
              )}
            </div>

            {showAdvancedYaml ? (
              <label className="block text-xs text-ink-3">
                values.yaml
                <textarea
                  value={valuesYaml || generatedYaml}
                  onChange={(e) => setValuesYaml(e.target.value)}
                  rows={8}
                  className="glass-input mt-1 font-mono"
                />
              </label>
            ) : (
              <pre className="glass-code-block-body text-[11px] text-ink-2 font-mono max-h-40 overflow-auto">
                {generatedYaml || '# No values generated'}
              </pre>
            )}
            {installMsg && <p className="text-sm text-ink-2">{installMsg}</p>}
            <div className="flex justify-end gap-2">
              <button type="button" onClick={() => setSelected(null)} className="rounded-lg border glass-divider px-4 py-2 text-sm">
                Cancel
              </button>
              <button
                type="button"
                onClick={() => void runInstall()}
                disabled={installing || !installCluster}
                className="btn-primary disabled:opacity-50"
              >
                {installing ? 'Installing…' : 'Deploy'}
              </button>
            </div>
          </div>
        )}
      </Modal>
    </div>
  );
}

export default withAuroraPage('helm', HelmCatalogPage);
