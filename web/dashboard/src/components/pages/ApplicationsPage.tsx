// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router';
import { Grid3X3, LayoutList, Plus, Search } from 'lucide-react';
import { apiFetchSettled } from '../../utils/api';
import { restartWorkload } from '../../utils/workloadActions';
import { useWorkspace } from '../../contexts/WorkspaceContext';
import { pathWithQuery, useQueryParam } from '../../utils/urlState';
import { viewToPath } from '../../utils/dashboardRoutes';
import { isK8sApplication, workspaceLabel } from '../../utils/k8sUx';
import type { WorkloadResponse } from '../../types/api';
import ApplicationCard from '../ApplicationCard';
import DataTable, { type DataTableColumn } from '../ui/DataTable';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import EmptyState from '../EmptyState';
import WorkloadDetail, { type DetailTab } from '../WorkloadDetail';
import { WorkloadContextBanner } from '../QueryContextBanner';
import { useAuth } from '../../contexts/AuthContext';

type ViewMode = 'grid' | 'list';

function ApplicationsPage({ refreshKey }: { refreshKey?: number } = {}) {
  const navigate = useNavigate();
  const { canMutate } = useAuth();
  const { workspace, setWorkspace } = useWorkspace();
  const [workloads, setWorkloads] = useState<WorkloadResponse[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [hasLoadedOnce, setHasLoadedOnce] = useState(false);
  const [search, setSearch] = useState('');
  const [viewMode, setViewMode] = useState<ViewMode>('grid');
  const [selected, setSelected] = useState<WorkloadResponse | null>(null);
  const [detailTab, setDetailTab] = useState<DetailTab>('overview');
  const [workloadParam, setWorkloadParam] = useQueryParam('workload');
  const [tabParam, setTabParam] = useQueryParam('tab');

  const load = useCallback(async () => {
    setLoading(true);
    setLoadFailed(false);
    const res = await apiFetchSettled<WorkloadResponse[]>('/workloads');
    if (!res.ok) {
      setLoadFailed(true);
      setWorkloads([]);
    } else {
      setWorkloads(res.data.filter(isK8sApplication));
    }
    setLoading(false);
    setHasLoadedOnce(true);
  }, [refreshKey]);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    if (!workloadParam || workloads.length === 0) return;
    const match = workloads.find((w) => w.name === workloadParam);
    if (!match) return;
    setSelected(match);
    const tab = tabParam as DetailTab | '';
    if (tab && ['overview', 'logs', 'topology', 'manifest', 'drift', 'scoring', 'events', 'trust'].includes(tab)) {
      setDetailTab(tab as DetailTab);
    }
  }, [workloadParam, tabParam, workloads]);

  function closeDetail() {
    setSelected(null);
    setWorkloadParam('');
    setTabParam('');
  }

  const workspaces = useMemo(() => {
    const set = new Set<string>();
    for (const w of workloads) {
      if (w.namespace) set.add(w.namespace);
    }
    return ['all', ...Array.from(set).sort()];
  }, [workloads]);

  const filtered = useMemo(() => {
    const q = search.trim().toLowerCase();
    return workloads.filter((w) => {
      if (workspace !== 'all' && w.namespace !== workspace) return false;
      if (!q) return true;
      const name = w.name.toLowerCase();
      return name.includes(q) || (w.namespace?.toLowerCase().includes(q) ?? false);
    });
  }, [workloads, workspace, search]);

  const appColumns: DataTableColumn<WorkloadResponse>[] = [
    {
      key: 'name',
      header: 'Application',
      render: (app) => <span className="font-medium text-foreground">{app.name}</span>,
    },
    {
      key: 'workspace',
      header: 'Workspace',
      render: (app) => <span className="text-muted">{workspaceLabel(app.namespace)}</span>,
    },
    {
      key: 'status',
      header: 'Status',
      render: (app) => <span className="text-muted">{app.status}</span>,
    },
    {
      key: 'kind',
      header: 'Kind',
      render: (app) => <span className="text-muted">{app.kind ?? 'Application'}</span>,
    },
    {
      key: 'actions',
      header: 'Actions',
      render: (app) => (
        <div onClick={(e) => e.stopPropagation()}>
          <button type="button" onClick={() => openApp(app)} className="text-primary text-xs hover:underline mr-3">
            Open
          </button>
          <button type="button" onClick={() => openApp(app, 'logs')} className="text-muted text-xs hover:underline">
            Logs
          </button>
        </div>
      ),
    },
  ];

  function openApp(app: WorkloadResponse, tab: DetailTab = 'overview') {
    setSelected(app);
    setDetailTab(tab);
    setWorkloadParam(app.name);
    if (tab !== 'overview') setTabParam(tab);
    else setTabParam('');
  }

  async function restartApp(app: WorkloadResponse) {
    const res = await restartWorkload(app);
    if (res.success) {
      window.dispatchEvent(new CustomEvent('aether-toast', { detail: { message: `Restarted ${app.name}`, type: 'success' } }));
      void load();
    }
  }

  if (loading && !hasLoadedOnce) return <PageLoading label="Loading applications…" />;
  if (loadFailed) {
    return (
      <PageLoadError
        title="Could not load applications"
        onRetry={() => void load()}
      />
    );
  }

  return (
    <div data-testid="applications-page">
      {workloadParam.trim() ? (
        <WorkloadContextBanner
          testId="applications-workload-context"
          workload={workloadParam.trim()}
          description="Application context"
        />
      ) : null}
      <section className="space-y-6">
      <div className="mb-6 flex flex-wrap items-center justify-end gap-2">
        <div className="flex flex-wrap items-center gap-2">
          <button
            type="button"
            onClick={() => navigate(pathWithQuery(viewToPath('editor'), { deploy: '1' }))}
            className="inline-flex items-center gap-2 btn-primary"
          >
            <Plus size={16} /> Deploy App
          </button>
        </div>
      </div>

      <div className="flex flex-col lg:flex-row gap-4 mb-6">
        <div className="flex flex-wrap gap-2 items-center flex-1">
          <span className="text-xs text-subtle uppercase tracking-wide mr-1">Workspace</span>
          {workspaces.map((ws) => (
            <button
              key={ws}
              type="button"
              onClick={() => setWorkspace(ws)}
              className={`glass-tab tab-chip capitalize ${workspace === ws ? 'glass-tab-active tab-chip-active' : ''}`}
            >
              {ws === 'all' ? 'All Workspaces' : workspaceLabel(ws)}
            </button>
          ))}
        </div>
        <div className="flex items-center gap-2">
          <div className="relative">
            <Search size={16} className="absolute left-3 top-1/2 -translate-y-1/2 text-subtle" />
            <input
              type="search"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="Search applications…"
              className="glass-input pl-9 w-56"
            />
          </div>
          <button
            type="button"
            onClick={() => setViewMode('grid')}
            className={`p-2 rounded-lg border ${viewMode === 'grid' ? 'border-primary text-primary' : 'glass-divider text-muted'}`}
            aria-label="Grid view"
          >
            <Grid3X3 size={16} />
          </button>
          <button
            type="button"
            onClick={() => setViewMode('list')}
            className={`p-2 rounded-lg border ${viewMode === 'list' ? 'border-primary text-primary' : 'glass-divider text-muted'}`}
            aria-label="List view"
          >
            <LayoutList size={16} />
          </button>
        </div>
      </div>

      {filtered.length === 0 ? (
        <EmptyState
          icon={<Plus size={48} />}
          title="No applications yet"
          description="Deploy your first Kubernetes application with the visual editor or import from Helm."
          action={
            <button
              type="button"
              onClick={() => navigate(pathWithQuery(viewToPath('editor'), { deploy: '1' }))}
              className="btn-primary"
            >
              Deploy App
            </button>
          }
        />
      ) : viewMode === 'grid' ? (
        <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-3 gap-4">
          {filtered.map((app) => (
            <ApplicationCard
              key={`${app.cluster ?? 'local'}-${app.namespace ?? 'default'}-${app.name}`}
              app={app}
              onOpen={() => openApp(app)}
              onLogs={() => openApp(app, 'logs')}
              onScale={() => openApp(app, 'overview')}
              onRestart={canMutate ? () => void restartApp(app) : undefined}
            />
          ))}
        </div>
      ) : (
        <DataTable<WorkloadResponse>
          items={filtered}
          getId={(app) => `${app.cluster ?? 'local'}-${app.namespace ?? 'default'}-${app.name}`}
          columns={appColumns}
          sortBySeverityDefault={false}
        />
      )}

      </section>

      {selected && (
        <WorkloadDetail
          key={`${selected.name}-${detailTab}`}
          workload={selected}
          onClose={closeDetail}
          onAction={() => void load()}
          initialTab={detailTab}
          canMutate={canMutate}
        />
      )}
    </div>
  );
}

export default withAuroraPage('applications', ApplicationsPage);
