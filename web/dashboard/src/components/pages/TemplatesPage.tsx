// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect, useCallback } from 'react';
import { withAuroraPage } from '../layout/AuroraPage';
import { Link, useNavigate } from 'react-router';
import { Rocket, Wand2, Inbox, Settings2, FileCode2 } from 'lucide-react';
import { apiFetchSettled, apiPost } from '../../utils/api';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery, useQueryParam, useWorkloadOrSearchFilter } from '../../utils/urlState';
import { markFirstDeploy } from '../../utils/onboardingState';
import PageToolbar from '../PageToolbar';
import CardGrid from '../CardGrid';
import EntityCard from '../EntityCard';
import Modal from '../Modal';
import EmptyState from '../EmptyState';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import { workloadJsonToYaml } from '../../utils/workloadYaml';
import { workloadNameFromSpec } from '../../utils/workloadNameFromSpec';
import { SearchQueryContextBanner, WorkloadScopedCrossLinks } from '../QueryContextBanner';
import type { Template } from '../../types/api';

function toast(message: string, type: 'success' | 'error') {
  window.dispatchEvent(new CustomEvent('aether-toast', { detail: { message, type } }));
}

function specPreview(spec: Record<string, unknown> | null): string {
  if (!spec) return '';
  return workloadJsonToYaml(spec);
}

function TemplatesPage({ refreshKey }: { refreshKey?: number } = {}) {
  const navigate = useNavigate();
  const [templates, setTemplates] = useState<Template[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [generateResult, setGenerateResult] = useState<string | null>(null);
  const [generateName, setGenerateName] = useState<string | null>(null);
  const [generatedSpec, setGeneratedSpec] = useState<Record<string, unknown> | null>(null);
  const [generateLoading, setGenerateLoading] = useState<string | null>(null);
  const [deployLoading, setDeployLoading] = useState(false);
  const [configureTemplate, setConfigureTemplate] = useState<string | null>(null);
  const [configureParam, setConfigureParam] = useQueryParam('configure');
  const [search, setSearch] = useWorkloadOrSearchFilter();
  const [params, setParams] = useState({
    workload_name: '',
    owner: '',
    project: '',
    registry: '',
    cpu: '',
    memory: '',
    port: '',
    replicas: '',
  });

  const load = useCallback(async () => {
    setLoading(true);
    setLoadFailed(false);
    const result = await apiFetchSettled<Template[]>('/templates');
    if (!result.ok) {
      setLoadFailed(true);
      setTemplates([]);
    } else {
      setTemplates(result.data);
    }
    setLoading(false);
  }, [refreshKey]);

  useEffect(() => {
    void load();
    const params = new URLSearchParams(window.location.search);
    const template = params.get('template');
    if (template) {
      params.delete('template');
      const qs = params.toString();
      window.history.replaceState(null, '', qs ? `${window.location.pathname}?${qs}` : window.location.pathname);
      void handleGenerate(template);
    }
  }, [load]);

  useEffect(() => {
    if (!configureParam) return;
    setConfigureTemplate(configureParam);
    setConfigureParam('');
  }, [configureParam, setConfigureParam]);

  async function handleGenerate(name: string, overrides?: Record<string, unknown>) {
    setGenerateLoading(name);
    const res = await apiPost<Record<string, unknown>>(`/templates/${name}`, overrides ?? {});
    if (res.success) {
      setGenerateResult(JSON.stringify(res.data ?? res, null, 2));
      setGenerateName(name);
      setGeneratedSpec(res.data ?? null);
      toast(`Template "${name}" generated successfully`, 'success');
    } else {
      toast(`Failed to generate template "${name}": ${res.error ?? 'unknown error'}`, 'error');
    }
    setGenerateLoading(null);
  }

  async function handleDeployGenerated() {
    if (!generatedSpec || deployLoading) return;
    setDeployLoading(true);
    const res = await apiPost('/workloads', { spec_yaml: workloadJsonToYaml(generatedSpec) });
    setDeployLoading(false);
    if (res.success) {
      markFirstDeploy();
      const wlName = workloadNameFromSpec(generatedSpec);
      toast(`Template "${generateName}" deployed successfully`, 'success');
      if (wlName) {
        navigate(pathWithQuery(viewToPath('workloads'), { workload: wlName }));
      }
    } else {
      toast(`Failed to deploy template "${generateName}": ${res.error ?? 'unknown error'}`, 'error');
    }
  }

  function buildOverrides() {
    return {
      workload_name: params.workload_name.trim() || undefined,
      owner: params.owner.trim() || undefined,
      project: params.project.trim() || undefined,
      registry: params.registry.trim() || undefined,
      cpu: params.cpu.trim() || undefined,
      memory: params.memory.trim() || undefined,
      port: params.port.trim() ? Number(params.port) : undefined,
      replicas: params.replicas.trim() ? Number(params.replicas) : undefined,
    };
  }

  const filtered = templates.filter(
    (t) =>
      t.name.toLowerCase().includes(search.toLowerCase()) ||
      t.description.toLowerCase().includes(search.toLowerCase())
  );

  if (loading && templates.length === 0 && !loadFailed) {
    return <PageLoading rows={4} />;
  }

  if (loadFailed) {
    return <PageLoadError title="Templates unavailable" onRetry={() => void load()} />;
  }

  return (
    <div>
      <SearchQueryContextBanner testId="templates-workload-context" query={search} entityLabel="templates">
        <WorkloadScopedCrossLinks workload={search} prefix="templates" />
        {search.trim() ? (
          <>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('editor'), { workload: search.trim() })}
              className="text-primary hover:underline"
              data-testid="templates-editor-link"
            >
              Editor →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('gitops'), { workload: search.trim() })}
              className="text-primary hover:underline"
              data-testid="templates-context-gitops-link"
            >
              GitOps →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('secrets'), { workload: search.trim() })}
              className="text-primary hover:underline"
              data-testid="templates-secrets-link"
            >
              Secrets →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('drift'), { workload: search.trim() })}
              className="text-primary hover:underline"
              data-testid="templates-context-drift-link"
            >
              Drift →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('compose'), { workload: search.trim() })}
              className="text-primary hover:underline"
              data-testid="templates-context-compose-link"
            >
              Compose →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('policy'), { workload: search.trim() })}
              className="text-primary hover:underline"
              data-testid="templates-context-policy-link"
            >
              Policy →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('intelligence'), { workload: search.trim(), tab: 'predictions' })}
              className="text-primary hover:underline"
              data-testid="templates-context-intelligence-link"
            >
              Intelligence →
            </Link>
          </>
        ) : null}
      </SearchQueryContextBanner>

      <section className="glass mb-6 p-6 sm:p-8">
      <div className="mb-4 flex flex-wrap gap-3">
        <button
          type="button"
          onClick={() =>
            navigate(
              pathWithQuery(viewToPath('workloads'), search.trim() ? { validate: '1', workload: search.trim() } : { validate: '1' }),
            )
          }
          className="text-xs text-primary hover:underline"
          data-testid="templates-policy-link"
        >
          Policy validate →
        </button>
        <button
          type="button"
          onClick={() => navigate(pathWithQuery(viewToPath('gitops'), search.trim() ? { workload: search.trim() } : {}))}
          className="text-xs text-primary hover:underline"
          data-testid="templates-gitops-link"
        >
          GitOps sync →
        </button>
        <button
          type="button"
          onClick={() => navigate(pathWithQuery(viewToPath('compose'), search.trim() ? { workload: search.trim() } : {}))}
          className="text-xs text-primary hover:underline"
          data-testid="templates-compose-link"
        >
          Compose import →
        </button>
      </div>
      <PageToolbar
        search={search}
        onSearchChange={setSearch}
        searchPlaceholder="Search templates…"
        onRefresh={() => void load()}
        refreshing={loading}
      />

      {templates.length === 0 ? (
        <EmptyState icon={<Inbox size={48} />} title="No templates" description="No templates are available" />
      ) : filtered.length === 0 ? (
        <EmptyState icon={<Inbox size={48} />} title="No matching templates" description="Try adjusting your search." />
      ) : (
        <CardGrid columns="compact" testId="templates-list">
          {filtered.map((t, i) => (
            <EntityCard
              key={t.name}
              index={i}
              testId={`template-card-${t.name}`}
              icon={<FileCode2 size={18} />}
              statusTone="sky"
              title={t.name}
              onClick={() => void handleGenerate(t.name)}
              body={
                <>
                  <p className="line-clamp-2 min-h-[2.5rem] text-[12px] leading-relaxed text-muted">{t.description}</p>
                  <div className="mt-3 flex flex-wrap gap-1.5">
                    <span className="rounded-md glass-inset-surface border glass-divider px-2 py-0.5 text-[11px] text-muted">CPU {t.default_cpu}</span>
                    <span className="rounded-md glass-inset-surface border glass-divider px-2 py-0.5 text-[11px] text-muted">Mem {t.default_memory}</span>
                  </div>
                </>
              }
              footer={
                <>
                  <button
                    type="button"
                    onClick={() => void handleGenerate(t.name)}
                    disabled={generateLoading === t.name}
                    className="inline-flex flex-1 items-center justify-center gap-1.5 rounded-lg px-2 py-1.5 text-[11px] font-medium text-muted transition hover:bg-primary/15 hover:text-primary disabled:opacity-50"
                  >
                    <Wand2 size={13} />
                    {generateLoading === t.name ? '…' : 'Generate'}
                  </button>
                  <button
                    type="button"
                    onClick={() => navigate(pathWithQuery(viewToPath('workloads'), { deploy: '1', template: t.name }))}
                    className="inline-flex flex-1 items-center justify-center gap-1.5 rounded-lg px-2 py-1.5 text-[11px] font-medium text-muted transition hover:bg-white/5 hover:text-foreground"
                  >
                    <FileCode2 size={13} />
                    Use
                  </button>
                  <button
                    type="button"
                    onClick={() => setConfigureTemplate(t.name)}
                    className="inline-flex flex-1 items-center justify-center gap-1.5 rounded-lg px-2 py-1.5 text-[11px] font-medium text-muted transition hover:bg-white/5 hover:text-foreground"
                  >
                    <Settings2 size={13} />
                    Config
                  </button>
                </>
              }
            />
          ))}
        </CardGrid>
      )}
      </section>


      <Modal
        isOpen={generateResult !== null}
        onClose={() => {
          setGenerateResult(null);
          setGenerateName(null);
          setGeneratedSpec(null);
          setDeployLoading(false);
        }}
        title={`Generated: ${generateName}`}
      >
        <div className="space-y-4">
          <div className="flex items-center justify-between gap-4 glass px-4 py-3">
            <div>
              <div className="text-sm font-medium text-foreground">Generated workload spec</div>
              <div className="text-xs text-subtle">Preview and deploy the generated template.</div>
            </div>
            <button
              type="button"
              onClick={() => void handleDeployGenerated()}
              disabled={deployLoading || !generatedSpec}
              data-testid="template-deploy-generated"
              className="inline-flex items-center gap-2 btn-primary disabled:opacity-50 shrink-0"
            >
              <Rocket size={14} />
              {deployLoading ? 'Deploying…' : 'Deploy'}
            </button>
            {generatedSpec ? (
              <button
                type="button"
                data-testid="template-open-editor"
                onClick={() => {
                  const name = workloadNameFromSpec(generatedSpec);
                  navigate(pathWithQuery(viewToPath('editor'), name ? { workload: name } : {}));
                }}
                className="inline-flex items-center gap-2 rounded-lg border glass-divider px-4 py-2 text-sm text-foreground glass-inset-hover shrink-0"
              >
                <FileCode2 size={14} />
                Open in editor
              </button>
            ) : null}
          </div>
          <div className="glass p-4">
            <h4 className="text-xs uppercase tracking-wider text-subtle mb-2">Preview</h4>
            <pre className="text-xs text-emerald-300 font-mono whitespace-pre-wrap overflow-auto max-h-48">
              {specPreview(generatedSpec)}
            </pre>
          </div>
          <details className="text-sm">
            <summary className="cursor-pointer text-muted hover:text-muted">Full JSON</summary>
            <pre className="mt-2 text-xs text-muted font-mono overflow-auto max-h-48">{generateResult ?? ''}</pre>
          </details>
        </div>
      </Modal>

      <Modal
        isOpen={configureTemplate !== null}
        onClose={() => setConfigureTemplate(null)}
        title={`Configure template: ${configureTemplate ?? ''}`}
      >
        <div data-testid="template-configure-modal" className="grid grid-cols-1 md:grid-cols-2 gap-3">
          {(
            [
              ['workload_name', 'Workload name'],
              ['owner', 'Owner'],
              ['project', 'Project'],
              ['registry', 'Registry'],
              ['cpu', 'CPU'],
              ['memory', 'Memory'],
              ['port', 'Port'],
              ['replicas', 'Replicas'],
            ] as const
          ).map(([key, label]) => (
            <input
              key={key}
              value={params[key]}
              onChange={(e) => setParams((current) => ({ ...current, [key]: e.target.value }))}
              placeholder={label}
              className="glass-input"
            />
          ))}
        </div>
        <div className="mt-4 flex justify-end gap-3">
          <button type="button" onClick={() => setConfigureTemplate(null)} className="px-4 py-2 rounded-lg text-sm text-muted glass-inset-hover">
            Cancel
          </button>
          <button
            type="button"
            onClick={() => {
              if (!configureTemplate) return;
              void handleGenerate(configureTemplate, buildOverrides());
              setConfigureTemplate(null);
            }}
            className="btn-primary"
          >
            Generate
          </button>
        </div>
      </Modal>
    </div>
  );
}

export default withAuroraPage('templates', TemplatesPage);
